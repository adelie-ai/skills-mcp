#![deny(warnings)]

// `SKILLS_MCP_READ_ONLY` hides the write tools from the listing and rejects
// calls to them, while the three read tools keep working.
//
// Everything runs inside ONE test function: the server resolves its mode and
// roots from process-global environment variables, so concurrent test threads
// must not race them. (Separate files under `tests/` are separate binaries
// and cannot race this one.)

use mcp_core::McpService;
use serde_json::json;
use skills_mcp::repo;
use skills_mcp::service::SkillsService;

/// Tiny in-tree temp-dir helper (mirrors `src/repo.rs`'s own) to avoid adding
/// a tempfile dev-dependency for these tests.
struct TempDir {
    path: std::path::PathBuf,
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn tempdir() -> TempDir {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock must give a nanos timestamp for a unique temp dir")
        .subsec_nanos();
    let path = std::env::temp_dir().join(format!(
        "skills-mcp-read-only-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("temp dir for read-only test must be creatable");
    TempDir { path }
}

fn tool_names(svc: &SkillsService) -> Vec<String> {
    let mut names: Vec<String> = svc.tools().into_iter().map(|t| t.name).collect();
    names.sort();
    names
}

#[tokio::test]
async fn read_only_env_hides_and_rejects_write_tools() {
    // -- Hermetic root setup. HOME is pointed at the temp dir so the real
    // ~/.agents/skills and ~/.claude/skills can never leak into list/search.
    let temp = tempdir();
    let root = temp.path.join("root");
    let skill = root.join("fixture-skill");
    std::fs::create_dir_all(&skill).expect("fixture skill dir must be creatable");
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: fixture-skill\ndescription: Fixture for the read-only test.\n---\n\nBody.\n",
    )
    .expect("fixture SKILL.md must be writable");
    let root_str = root.display().to_string();
    // SAFETY: this is the only test in this binary, so nothing else reads or
    // writes the process environment concurrently. READ_ONLY_ENV is unique to
    // this binary; HOME/ROOTS/WRITE_ROOT are read by repo fns this test calls.
    unsafe {
        std::env::set_var("HOME", temp.path.display().to_string());
        std::env::set_var(repo::ROOTS_ENV, &root_str);
        std::env::set_var(repo::WRITE_ROOT_ENV, &root_str);
        std::env::remove_var(repo::READ_ONLY_ENV);
    }

    let svc = SkillsService;
    let all = tool_names(&svc);
    assert_eq!(
        all.len(),
        6,
        "default mode must advertise all six tools, got {all:?}"
    );

    // -- Falsy values keep the write tools.
    for falsy in ["0", "false", "no", ""] {
        // SAFETY: same single-test invariant as above.
        unsafe { std::env::set_var(repo::READ_ONLY_ENV, falsy) };
        assert_eq!(
            tool_names(&svc).len(),
            6,
            "READ_ONLY={falsy:?} must not hide tools"
        );
    }

    // -- Truthy values hide exactly the three write tools.
    for truthy in ["1", "true", "TRUE", "yes"] {
        // SAFETY: same single-test invariant as above.
        unsafe { std::env::set_var(repo::READ_ONLY_ENV, truthy) };
        assert_eq!(
            tool_names(&svc),
            vec![
                "skills_get_skill".to_string(),
                "skills_list_skills".to_string(),
                "skills_search_skills".to_string(),
            ],
            "READ_ONLY={truthy:?} must advertise only the read tools"
        );
    }

    // -- Hidden tools fail like tools the server never had; reads still work.
    // SAFETY: same single-test invariant as above.
    unsafe { std::env::set_var(repo::READ_ONLY_ENV, "1") };
    for hidden in [
        "skills_create_skill",
        "skills_update_skill",
        "skills_delete_skill",
    ] {
        let err = svc
            .call_tool(hidden, &json!({"name": "anything"}))
            .await
            .expect_err(&format!("{hidden} must be rejected in read-only mode"));
        assert!(
            format!("{err:?}").contains("unknown tool"),
            "{hidden} must fail as an unknown tool, got {err:?}"
        );
    }
    let list = svc
        .call_tool("skills_list_skills", &json!({}))
        .await
        .expect("list must keep working in read-only mode");
    assert!(!list.is_error, "list reply must not be an error");
    let get = svc
        .call_tool("skills_get_skill", &json!({"name": "fixture-skill"}))
        .await
        .expect("get must keep working in read-only mode");
    assert!(!get.is_error, "get reply must not be an error");

    // -- Clearing the flag restores the full listing.
    // SAFETY: same single-test invariant as above.
    unsafe { std::env::remove_var(repo::READ_ONLY_ENV) };
    assert_eq!(
        tool_names(&svc).len(),
        6,
        "clearing READ_ONLY must restore all six tools"
    );
}
