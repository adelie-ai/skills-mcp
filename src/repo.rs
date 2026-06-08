#![deny(warnings)]

// On-disk skill repository following the Anthropic Agent Skills format:
//
//   <root>/<name>/SKILL.md
//
// SKILL.md begins with a YAML frontmatter block delimited by `---` lines:
//
//   ---
//   name: my-skill
//   description: One-line trigger description for the agent.
//   tags: [optional, tag, list]
//   ---
//   <markdown body>
//
// Any other files in the skill directory are reported as `attachments` so
// downstream consumers (e.g. Adelie's knowledge-base ingester) can find
// supporting scripts/assets.

use crate::error::{Result, SkillsError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// File name used by every skill directory.
pub const SKILL_FILE: &str = "SKILL.md";

/// Environment variable used to add extra skill roots (colon-separated).
pub const ROOTS_ENV: &str = "SKILLS_MCP_ROOTS";

/// Validate a skill name before it is joined onto a root directory.
///
/// Why: every filesystem operation derives a path via `root.join(name)`. An
/// unsanitised `name` such as `../../etc` or `/etc/motd` would escape the
/// configured root and let a caller read, create, rename, or delete arbitrary
/// paths. We require the trimmed name to be a single non-empty path component
/// that is `Component::Normal` (rejecting `.`, `..`, absolute prefixes, and
/// any embedded `/` or `\` separator).
pub fn validate_skill_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(SkillsError::InvalidInput("name must not be empty".into()).into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(SkillsError::InvalidInput(format!(
            "skill name must not contain path separators: {trimmed:?}"
        ))
        .into());
    }
    let mut components = Path::new(trimmed).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(()),
        _ => Err(SkillsError::InvalidInput(format!(
            "skill name must be a single normal path component, not {trimmed:?}"
        ))
        .into()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillFrontmatter {
    /// Stable identifier for the skill. Must match the directory name.
    pub name: String,
    /// Short description (1-2 sentences) telling the agent when to use this skill.
    pub description: String,
    /// Optional tags for filtering and discovery.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    /// Absolute path to SKILL.md.
    pub path: String,
    /// Absolute path to the root that contains this skill.
    pub root: String,
    /// Names of additional files in the skill directory (relative to the directory).
    pub attachments: Vec<String>,
}

impl SkillSummary {
    /// Render this summary for an LLM-facing list/search response. The absolute
    /// `path`/`root` fields are omitted unless `include_paths` is set, to save
    /// tokens and avoid leaking the host filesystem layout.
    pub fn to_view(&self, include_paths: bool) -> serde_json::Value {
        let mut value = serde_json::json!({
            "name": self.name,
            "description": self.description,
            "tags": self.tags,
            "attachments": self.attachments,
        });
        if include_paths && let Some(obj) = value.as_object_mut() {
            obj.insert("path".into(), serde_json::Value::String(self.path.clone()));
            obj.insert("root".into(), serde_json::Value::String(self.root.clone()));
        }
        value
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SkillDetail {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    /// Markdown body of SKILL.md (everything after the frontmatter block).
    pub content: String,
    pub path: String,
    pub root: String,
    pub attachments: Vec<String>,
}

/// Discover skill roots in lookup order. Only existing directories are
/// returned. Order: `$SKILLS_MCP_ROOTS` entries (left to right), then
/// `~/.agents/skills`, then `~/.claude/skills`.
pub fn lookup_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(env) = std::env::var(ROOTS_ENV) {
        for part in env.split(':').filter(|p| !p.is_empty()) {
            let expanded = shellexpand::full(part)
                .map(|s| PathBuf::from(s.as_ref()))
                .unwrap_or_else(|_| PathBuf::from(part));
            if expanded.is_dir() {
                roots.push(expanded);
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let candidates = [home.join(".agents/skills"), home.join(".claude/skills")];
        for c in candidates {
            if c.is_dir() && !roots.contains(&c) {
                roots.push(c);
            }
        }
    }
    // The write root must also be a read root, otherwise skills created via
    // `write_new` (e.g. into a $SKILLS_MCP_WRITE_ROOT override) would be
    // invisible to list/find/search.
    let write_root = default_write_root();
    if write_root.is_dir() && !roots.contains(&write_root) {
        roots.push(write_root);
    }
    roots
}

/// Environment variable that overrides where `skills_create_skill` writes.
pub const WRITE_ROOT_ENV: &str = "SKILLS_MCP_WRITE_ROOT";

/// Default root used when creating new skills. Created on demand by `write`.
/// `$SKILLS_MCP_WRITE_ROOT` overrides the default; useful when
/// `~/.agents/skills` is owned by a package-management tool and not
/// writable by the current user.
pub fn default_write_root() -> PathBuf {
    if let Ok(env) = std::env::var(WRITE_ROOT_ENV) {
        return shellexpand::full(&env)
            .map(|s| PathBuf::from(s.as_ref()))
            .unwrap_or_else(|_| PathBuf::from(env));
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".agents/skills")
}

/// List every skill across every configured root. Skills with a malformed
/// frontmatter are skipped with a warning instead of failing the whole list.
pub fn list_all() -> Vec<SkillSummary> {
    let mut out = Vec::new();
    for root in lookup_roots() {
        for entry in walkdir::WalkDir::new(&root)
            .min_depth(2)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() != SKILL_FILE {
                continue;
            }
            match read_summary(entry.path(), &root) {
                Ok(s) => out.push(s),
                Err(e) => {
                    eprintln!("skills-mcp: skipping {}: {}", entry.path().display(), e);
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Locate a skill by name across all configured roots. Returns the first match.
/// Returns `None` for names that fail [`validate_skill_name`] so a traversal
/// attempt can never resolve to a real file outside a root.
pub fn find(name: &str) -> Option<(PathBuf, PathBuf)> {
    if validate_skill_name(name).is_err() {
        return None;
    }
    let name = name.trim();
    for root in lookup_roots() {
        let dir = root.join(name);
        let path = dir.join(SKILL_FILE);
        if path.is_file() {
            return Some((path, root));
        }
    }
    None
}

/// Read a full skill (frontmatter + body + attachments) by name.
pub fn read(name: &str) -> Result<SkillDetail> {
    let (path, root) = find(name).ok_or_else(|| SkillsError::NotFound(name.to_string()))?;
    let raw = fs::read_to_string(&path).map_err(|e| {
        SkillsError::StorageError(format!("failed to read {}: {}", path.display(), e))
    })?;
    let (fm, body) = parse_skill_md(&raw)?;
    Ok(SkillDetail {
        name: fm.name,
        description: fm.description,
        tags: fm.tags,
        content: body,
        attachments: collect_attachments(&path),
        path: path.display().to_string(),
        root: root.display().to_string(),
    })
}

/// Read just the frontmatter and attachment list (no body) for the listing view.
fn read_summary(path: &Path, root: &Path) -> Result<SkillSummary> {
    let raw = fs::read_to_string(path).map_err(|e| {
        SkillsError::StorageError(format!("failed to read {}: {}", path.display(), e))
    })?;
    let (fm, _body) = parse_skill_md(&raw)?;
    Ok(SkillSummary {
        name: fm.name,
        description: fm.description,
        tags: fm.tags,
        attachments: collect_attachments(path),
        path: path.display().to_string(),
        root: root.display().to_string(),
    })
}

/// Write a new skill. Errors if a skill with the same name already exists
/// in any configured root.
pub fn write_new(name: &str, fm: &SkillFrontmatter, body: &str) -> Result<SkillDetail> {
    validate_skill_name(name)?;
    let name = name.trim();
    if fm.description.trim().is_empty() {
        return Err(SkillsError::InvalidInput("description must not be empty".into()).into());
    }
    if find(name).is_some() {
        return Err(SkillsError::AlreadyExists(name.to_string()).into());
    }
    let root = default_write_root();
    fs::create_dir_all(&root).map_err(|e| {
        SkillsError::StorageError(format!(
            "failed to create write root {}: {} \
             (override with $SKILLS_MCP_WRITE_ROOT if the default is read-only)",
            root.display(),
            e
        ))
    })?;
    let dir = root.join(name);
    fs::create_dir_all(&dir).map_err(|e| {
        SkillsError::StorageError(format!(
            "failed to create {}: {} \
             (override with $SKILLS_MCP_WRITE_ROOT if the default is read-only)",
            dir.display(),
            e
        ))
    })?;
    let path = dir.join(SKILL_FILE);
    let serialised = render_skill_md(fm, body)?;
    atomic_write(&path, &serialised)?;
    Ok(SkillDetail {
        name: fm.name.clone(),
        description: fm.description.clone(),
        tags: fm.tags.clone(),
        content: body.to_string(),
        attachments: collect_attachments(&path),
        path: path.display().to_string(),
        root: root.display().to_string(),
    })
}

/// Overwrite an existing skill in place. Only fields set in `patch` are
/// changed; the rest are read from disk.
pub fn write_update(name: &str, patch: UpdatePatch) -> Result<SkillDetail> {
    let current = read(name)?;
    let new_name = match patch.name {
        Some(n) => {
            validate_skill_name(&n)?;
            n.trim().to_string()
        }
        None => current.name.clone(),
    };
    if new_name != current.name && find(&new_name).is_some() {
        return Err(SkillsError::AlreadyExists(new_name).into());
    }
    let fm = SkillFrontmatter {
        name: new_name.clone(),
        description: patch.description.unwrap_or(current.description),
        tags: patch.tags.unwrap_or(current.tags),
    };
    let body = patch.content.unwrap_or(current.content);
    let serialised = render_skill_md(&fm, &body)?;
    let current_path = PathBuf::from(&current.path);
    let current_dir = current_path.parent().ok_or_else(|| {
        SkillsError::StorageError(format!(
            "{} has no parent directory",
            current_path.display()
        ))
    })?;

    let final_dir = if new_name != current.name {
        let root = PathBuf::from(&current.root);
        let target_dir = root.join(&new_name);
        fs::rename(current_dir, &target_dir).map_err(|e| {
            SkillsError::StorageError(format!(
                "failed to rename {} -> {}: {}",
                current_dir.display(),
                target_dir.display(),
                e
            ))
        })?;
        target_dir
    } else {
        current_dir.to_path_buf()
    };
    let path = final_dir.join(SKILL_FILE);
    atomic_write(&path, &serialised)?;
    Ok(SkillDetail {
        name: fm.name,
        description: fm.description,
        tags: fm.tags,
        content: body,
        attachments: collect_attachments(&path),
        path: path.display().to_string(),
        root: current.root,
    })
}

/// Patch shape for `write_update`. Each field is `Some(_)` only if the
/// caller wants to change it.
pub struct UpdatePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Delete a skill directory recursively. Returns the deleted skill's
/// summary so the caller can confirm what was removed.
pub fn delete(name: &str) -> Result<SkillSummary> {
    validate_skill_name(name)?;
    let (path, root) = find(name).ok_or_else(|| SkillsError::NotFound(name.to_string()))?;
    let summary = read_summary(&path, &root)?;
    let dir = path.parent().ok_or_else(|| {
        SkillsError::StorageError(format!("{} has no parent directory", path.display()))
    })?;
    fs::remove_dir_all(dir).map_err(|e| {
        SkillsError::StorageError(format!("failed to remove {}: {}", dir.display(), e))
    })?;
    Ok(summary)
}

/// Full-text search across name / description / tags / body.
pub fn search(query: &str, required_tags: &[String]) -> Vec<SkillSummary> {
    let needle = query.to_lowercase();
    let mut out = Vec::new();
    for s in list_all() {
        if !required_tags.is_empty() && !required_tags.iter().any(|t| s.tags.iter().any(|x| x == t))
        {
            continue;
        }
        if matches_summary(&s, &needle) {
            out.push(s);
        } else if let Ok(detail) = read(&s.name)
            && detail.content.to_lowercase().contains(&needle)
        {
            out.push(s);
        }
    }
    out
}

fn matches_summary(s: &SkillSummary, needle: &str) -> bool {
    if s.name.to_lowercase().contains(needle) {
        return true;
    }
    if s.description.to_lowercase().contains(needle) {
        return true;
    }
    s.tags.iter().any(|t| t.to_lowercase().contains(needle))
}

fn collect_attachments(skill_md_path: &Path) -> Vec<String> {
    let Some(dir) = skill_md_path.parent() else {
        return Vec::new();
    };
    let mut out: Vec<String> = walkdir::WalkDir::new(dir)
        .min_depth(1)
        // Bound recursion so a pathological skill directory can't make an
        // attachment listing walk an unbounded tree.
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path() != skill_md_path && e.file_type().is_file())
        .filter_map(|e| {
            e.path()
                .strip_prefix(dir)
                .ok()
                .map(|p| p.display().to_string())
        })
        .collect();
    out.sort();
    out
}

/// Parse a SKILL.md file. Returns the parsed frontmatter and the body.
///
/// The frontmatter is the first YAML block delimited by `---` lines. The
/// closing `---` must be the only content on its line. Files without a
/// frontmatter block return an error.
fn parse_skill_md(raw: &str) -> Result<(SkillFrontmatter, String)> {
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let trimmed = raw.trim_start();
    let after_open = trimmed
        .strip_prefix("---\n")
        .or_else(|| trimmed.strip_prefix("---\r\n"))
        .ok_or_else(|| {
            SkillsError::InvalidInput(
                "SKILL.md is missing a leading `---` frontmatter block".into(),
            )
        })?;
    let close = find_close_fence(after_open).ok_or_else(|| {
        SkillsError::InvalidInput("SKILL.md frontmatter is missing a closing `---`".into())
    })?;
    let (yaml, rest) = after_open.split_at(close);
    let body = rest
        .trim_start_matches("---\n")
        .trim_start_matches("---\r\n")
        .trim_start_matches("---")
        .trim_start_matches('\n');
    let fm: SkillFrontmatter = serde_yaml_ng::from_str(yaml)
        .map_err(|e| SkillsError::InvalidInput(format!("invalid SKILL.md frontmatter: {e}")))?;
    Ok((fm, body.to_string()))
}

/// Find the byte offset of the line that closes the frontmatter (a line
/// containing only `---`). Returns the offset where that line starts.
fn find_close_fence(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut line_start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let line = &s[line_start..i];
            let line = line.trim_end_matches('\r');
            if line == "---" {
                return Some(line_start);
            }
            line_start = i + 1;
        }
        i += 1;
    }
    // Tail without a trailing newline.
    let line = s[line_start..].trim_end_matches('\r');
    if line == "---" {
        Some(line_start)
    } else {
        None
    }
}

fn render_skill_md(fm: &SkillFrontmatter, body: &str) -> Result<String> {
    let yaml = serde_yaml_ng::to_string(fm)
        .map_err(|e| SkillsError::StorageError(format!("failed to serialise frontmatter: {e}")))?;
    let body_trimmed = body.trim_end();
    Ok(format!("---\n{yaml}---\n\n{body_trimmed}\n"))
}

fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents).map_err(|e| {
        SkillsError::StorageError(format!("failed to write {}: {}", tmp.display(), e))
    })?;
    fs::rename(&tmp, path).map_err(|e| {
        SkillsError::StorageError(format!(
            "failed to rename {} -> {}: {}",
            tmp.display(),
            path.display(),
            e
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SkillsMcpError;
    use std::sync::{Mutex, MutexGuard};

    /// Serialise tests that mutate the process-global skill-root env vars so
    /// they don't race each other under the default parallel test runner.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Acquire the env lock, recovering from a poisoned mutex (a panic in
    /// another env-mutating test must not cascade into spurious failures).
    fn env_guard() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn validate_skill_name_accepts_benign() {
        validate_skill_name("my-skill").expect("plain name is valid");
        validate_skill_name("nested-ok_123").expect("alnum/dash/underscore is valid");
    }

    #[test]
    fn validate_skill_name_rejects_traversal() {
        assert!(validate_skill_name("../escape").is_err());
        assert!(validate_skill_name("../../../tmp/x").is_err());
        assert!(validate_skill_name("a/b").is_err());
        assert!(validate_skill_name("a\\b").is_err());
        assert!(validate_skill_name("/absolute").is_err());
        assert!(validate_skill_name("/etc/motd").is_err());
        assert!(validate_skill_name(".").is_err());
        assert!(validate_skill_name("..").is_err());
        assert!(validate_skill_name("").is_err());
        assert!(validate_skill_name("   ").is_err());
    }

    #[test]
    fn find_rejects_traversal() {
        let _g = env_guard();
        let temp = tempdir();
        unsafe {
            std::env::set_var(ROOTS_ENV, temp.path().display().to_string());
        }
        assert!(find("../../../tmp/x").is_none());
        assert!(find("/etc/motd").is_none());
        unsafe {
            std::env::remove_var(ROOTS_ENV);
        }
    }

    #[test]
    fn write_new_rejects_traversal() {
        let _g = env_guard();
        let temp = tempdir();
        unsafe {
            std::env::set_var(WRITE_ROOT_ENV, temp.path().display().to_string());
            std::env::set_var(ROOTS_ENV, temp.path().display().to_string());
        }
        let fm = SkillFrontmatter {
            name: "../escape".into(),
            description: "d".into(),
            tags: vec![],
        };
        let err = write_new("../escape", &fm, "body").unwrap_err();
        assert!(matches!(
            err,
            SkillsMcpError::Skills(SkillsError::InvalidInput(_))
        ));
        // Nothing escaped the configured root.
        assert!(!temp.path().parent().unwrap().join("escape").exists());
        unsafe {
            std::env::remove_var(WRITE_ROOT_ENV);
            std::env::remove_var(ROOTS_ENV);
        }
    }

    #[test]
    fn write_update_rejects_traversal_new_name() {
        let _g = env_guard();
        let temp = tempdir();
        unsafe {
            std::env::set_var(WRITE_ROOT_ENV, temp.path().display().to_string());
            std::env::set_var(ROOTS_ENV, temp.path().display().to_string());
        }
        let fm = SkillFrontmatter {
            name: "real".into(),
            description: "d".into(),
            tags: vec![],
        };
        write_new("real", &fm, "body").expect("create real skill");
        let err = write_update(
            "real",
            UpdatePatch {
                name: Some("/etc/motd".into()),
                description: None,
                content: None,
                tags: None,
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            SkillsMcpError::Skills(SkillsError::InvalidInput(_))
        ));
        unsafe {
            std::env::remove_var(WRITE_ROOT_ENV);
            std::env::remove_var(ROOTS_ENV);
        }
    }

    #[test]
    fn delete_rejects_traversal() {
        let _g = env_guard();
        let temp = tempdir();
        unsafe {
            std::env::set_var(ROOTS_ENV, temp.path().display().to_string());
        }
        let err = delete("../../../tmp/x").unwrap_err();
        assert!(matches!(
            err,
            SkillsMcpError::Skills(SkillsError::InvalidInput(_))
                | SkillsMcpError::Skills(SkillsError::NotFound(_))
        ));
        unsafe {
            std::env::remove_var(ROOTS_ENV);
        }
    }

    #[test]
    fn write_root_is_searchable() {
        let _g = env_guard();
        let temp = tempdir();
        // Only the write-root env is set; the skill must still be listable.
        unsafe {
            std::env::set_var(WRITE_ROOT_ENV, temp.path().display().to_string());
            std::env::remove_var(ROOTS_ENV);
        }
        let fm = SkillFrontmatter {
            name: "in-write-root".into(),
            description: "d".into(),
            tags: vec![],
        };
        write_new("in-write-root", &fm, "body").expect("create in write root");
        let names: Vec<String> = list_all().into_iter().map(|s| s.name).collect();
        assert!(
            names.contains(&"in-write-root".to_string()),
            "skill created in write-root should appear in list_all: {names:?}"
        );
        unsafe {
            std::env::remove_var(WRITE_ROOT_ENV);
        }
    }

    #[test]
    fn summary_view_omits_paths_by_default() {
        let s = SkillSummary {
            name: "n".into(),
            description: "d".into(),
            tags: vec!["t".into()],
            path: "/secret/n/SKILL.md".into(),
            root: "/secret".into(),
            attachments: vec![],
        };
        let default = s.to_view(false);
        assert!(default.get("path").is_none());
        assert!(default.get("root").is_none());
        assert_eq!(default["name"], "n");

        let with_paths = s.to_view(true);
        assert_eq!(with_paths["path"], "/secret/n/SKILL.md");
        assert_eq!(with_paths["root"], "/secret");
    }

    #[test]
    fn parse_full_frontmatter() {
        let raw = "---\nname: foo\ndescription: bar\ntags: [a, b]\n---\nbody here\n";
        let (fm, body) = parse_skill_md(raw).unwrap();
        assert_eq!(fm.name, "foo");
        assert_eq!(fm.description, "bar");
        assert_eq!(fm.tags, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(body, "body here\n");
    }

    #[test]
    fn parse_tolerates_bom_and_leading_whitespace() {
        let raw = "\u{feff}\n---\nname: x\ndescription: y\n---\nz";
        let (fm, body) = parse_skill_md(raw).unwrap();
        assert_eq!(fm.name, "x");
        assert_eq!(body, "z");
    }

    #[test]
    fn parse_rejects_missing_frontmatter() {
        assert!(parse_skill_md("plain markdown only").is_err());
    }

    #[test]
    fn parse_rejects_unterminated_frontmatter() {
        assert!(parse_skill_md("---\nname: x\ndescription: y\nno closing fence").is_err());
    }

    #[test]
    fn render_roundtrips() {
        let fm = SkillFrontmatter {
            name: "foo".into(),
            description: "trigger description".into(),
            tags: vec!["t1".into()],
        };
        let body = "## Section\n\nbody text";
        let rendered = render_skill_md(&fm, body).unwrap();
        let (parsed_fm, parsed_body) = parse_skill_md(&rendered).unwrap();
        assert_eq!(parsed_fm.name, fm.name);
        assert_eq!(parsed_fm.description, fm.description);
        assert_eq!(parsed_fm.tags, fm.tags);
        assert_eq!(parsed_body.trim(), body);
    }

    #[test]
    fn write_and_read_round_trip() {
        let _g = env_guard();
        let temp = tempdir();
        unsafe {
            std::env::set_var(ROOTS_ENV, temp.path().display().to_string());
        }
        let fm = SkillFrontmatter {
            name: "demo".into(),
            description: "demo desc".into(),
            tags: vec![],
        };
        // Pre-create the demo dir inside our temp root so write_new sees
        // the temp root as the only candidate write target.
        let root_override = temp.path().to_path_buf();
        fs::create_dir_all(root_override.join("demo")).unwrap();
        let path = root_override.join("demo").join(SKILL_FILE);
        atomic_write(&path, &render_skill_md(&fm, "body").unwrap()).unwrap();

        let got = read("demo").unwrap();
        assert_eq!(got.name, "demo");
        assert_eq!(got.description, "demo desc");
        assert_eq!(got.content.trim(), "body");

        unsafe {
            std::env::remove_var(ROOTS_ENV);
        }
    }

    /// Tiny in-tree temp-dir helper to avoid pulling in the tempfile crate
    /// for a single test.
    fn tempdir() -> TempDir {
        let path = std::env::temp_dir().join(format!(
            "skills-mcp-test-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn rand_suffix() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0)
    }
}
