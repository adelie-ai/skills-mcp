#![deny(warnings)]

// JSON file-backed in-memory skill store

use crate::error::{Result, SkillsError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The kind of a skill entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillKind {
    /// A reusable code snippet in a specific language.
    Code,
    /// A natural-language how-to guide for an LLM agent.
    Howto,
}

impl std::fmt::Display for SkillKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillKind::Code => write!(f, "code"),
            SkillKind::Howto => write!(f, "howto"),
        }
    }
}

impl std::str::FromStr for SkillKind {
    type Err = SkillsError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "code" => Ok(SkillKind::Code),
            "howto" => Ok(SkillKind::Howto),
            other => Err(SkillsError::InvalidInput(format!(
                "Unknown skill kind '{}'. Valid values: code, howto",
                other
            ))),
        }
    }
}

/// A single skill entry (either a code snippet or a how-to document).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Unique UUID v4 identifier.
    pub id: String,
    /// Human-friendly name (must be unique).
    pub name: String,
    /// Whether this is a code snippet or a how-to document.
    pub kind: SkillKind,
    /// Programming language (only meaningful for `kind = code`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Short one-line description of what this skill does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The actual code or natural-language instructions.
    pub content: String,
    /// Arbitrary tags for filtering/discovery.
    #[serde(default)]
    pub tags: Vec<String>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
}

/// Serialised form of the whole database file.
#[derive(Debug, Serialize, Deserialize, Default)]
struct DbFile {
    skills: Vec<Skill>,
}

/// In-memory skill store backed by a JSON file.
pub struct SkillDb {
    /// Path to the backing JSON file.
    path: PathBuf,
    /// Skills keyed by ID for O(1) lookup.
    skills: HashMap<String, Skill>,
}

impl SkillDb {
    /// Open (or create) the database at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        let expanded = shellexpand::full(&path.to_string_lossy())
            .map_err(|e| {
                SkillsError::StorageError(format!("Failed to expand path: {}", e))
            })
            .map(|s| PathBuf::from(s.as_ref()))?;

        let skills = if expanded.exists() {
            let raw = fs::read_to_string(&expanded)
                .map_err(|e| SkillsError::StorageError(format!("Failed to read db: {}", e)))?;
            let db: DbFile = serde_json::from_str(&raw)
                .map_err(|e| SkillsError::StorageError(format!("Failed to parse db: {}", e)))?;
            db.skills.into_iter().map(|s| (s.id.clone(), s)).collect()
        } else {
            // Create parent dirs so the first save succeeds.
            if let Some(parent) = expanded.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    SkillsError::StorageError(format!("Failed to create db directory: {}", e))
                })?;
            }
            HashMap::new()
        };

        Ok(Self { path: expanded, skills })
    }

    /// Persist the current in-memory state to disk.
    fn save(&self) -> Result<()> {
        let db = DbFile {
            skills: {
                let mut v: Vec<Skill> = self.skills.values().cloned().collect();
                v.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                v
            },
        };
        let json = serde_json::to_string_pretty(&db)
            .map_err(|e| SkillsError::StorageError(format!("Failed to serialise db: {}", e)))?;
        // Atomic write: write to a temp file next to the target, then rename.
        let tmp_path = self.path.with_extension("tmp");
        fs::write(&tmp_path, &json)
            .map_err(|e| SkillsError::StorageError(format!("Failed to write db temp: {}", e)))?;
        fs::rename(&tmp_path, &self.path)
            .map_err(|e| SkillsError::StorageError(format!("Failed to rename db: {}", e)))?;
        Ok(())
    }

    /// Create a new skill. Returns the created skill.
    ///
    /// Errors if a skill with the same name already exists.
    pub fn create(&mut self, req: CreateSkillRequest) -> Result<Skill> {
        // Validate required fields
        if req.name.trim().is_empty() {
            return Err(SkillsError::InvalidInput("name must not be empty".to_string()).into());
        }
        if req.content.trim().is_empty() {
            return Err(SkillsError::InvalidInput("content must not be empty".to_string()).into());
        }

        // Enforce unique names
        if self.skills.values().any(|s| s.name == req.name) {
            return Err(SkillsError::AlreadyExists(req.name.clone()).into());
        }

        // Warn if language provided for howto (store it anyway)
        let now = Utc::now().to_rfc3339();
        let skill = Skill {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            kind: req.kind,
            language: req.language,
            description: req.description,
            content: req.content,
            tags: req.tags.unwrap_or_default(),
            created_at: now.clone(),
            updated_at: now,
        };
        self.skills.insert(skill.id.clone(), skill.clone());
        self.save()?;
        Ok(skill)
    }

    /// Retrieve a skill by ID or by exact name.
    pub fn get(&self, id_or_name: &str) -> Result<Skill> {
        // Try ID first
        if let Some(s) = self.skills.get(id_or_name) {
            return Ok(s.clone());
        }
        // Fall back to name search
        self.skills
            .values()
            .find(|s| s.name == id_or_name)
            .cloned()
            .ok_or_else(|| SkillsError::NotFound(id_or_name.to_string()).into())
    }

    /// Update fields of an existing skill. Returns the updated skill.
    pub fn update(&mut self, id_or_name: &str, req: UpdateSkillRequest) -> Result<Skill> {
        // Resolve to ID
        let id = self.get(id_or_name)?.id;

        // Ensure new name (if given) is not already taken
        if let Some(ref new_name) = req.name
            && self
                .skills
                .values()
                .any(|s| &s.name == new_name && s.id != id)
        {
            return Err(SkillsError::AlreadyExists(new_name.clone()).into());
        }

        let skill = self
            .skills
            .get_mut(&id)
            .ok_or_else(|| SkillsError::NotFound(id.clone()))?;
        if let Some(name) = req.name {
            skill.name = name;
        }
        if let Some(kind) = req.kind {
            skill.kind = kind;
        }
        if let Some(lang) = req.language {
            skill.language = lang;
        }
        if let Some(desc) = req.description {
            skill.description = desc;
        }
        if let Some(content) = req.content {
            if content.trim().is_empty() {
                return Err(
                    SkillsError::InvalidInput("content must not be empty".to_string()).into(),
                );
            }
            skill.content = content;
        }
        if let Some(tags) = req.tags {
            skill.tags = tags;
        }
        skill.updated_at = Utc::now().to_rfc3339();
        let updated = skill.clone();
        self.save()?;
        Ok(updated)
    }

    /// Delete a skill by ID or name. Returns the deleted skill.
    pub fn delete(&mut self, id_or_name: &str) -> Result<Skill> {
        let id = self.get(id_or_name)?.id;
        let removed = self
            .skills
            .remove(&id)
            .ok_or_else(|| SkillsError::NotFound(id.clone()))?;
        self.save()?;
        Ok(removed)
    }

    /// List skills with optional filtering.
    pub fn list(&self, kind: Option<&SkillKind>, tags: &[String]) -> Vec<Skill> {
        let mut results: Vec<Skill> = self
            .skills
            .values()
            .filter(|s| {
                if let Some(k) = kind
                    && &s.kind != k
                {
                    return false;
                }
                if !tags.is_empty() && !tags.iter().any(|t| s.tags.contains(t)) {
                    return false;
                }
                true
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        results
    }

    /// Search skills by a query string.
    ///
    /// Matches (case-insensitive) against name, description, content, and tags.
    pub fn search(&self, query: &str, kind: Option<&SkillKind>) -> Vec<Skill> {
        let q = query.to_lowercase();
        let mut results: Vec<Skill> = self
            .skills
            .values()
            .filter(|s| {
                if let Some(k) = kind
                    && &s.kind != k
                {
                    return false;
                }
                s.name.to_lowercase().contains(&q)
                    || s.description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
                    || s.content.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || s.language
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
            })
            .cloned()
            .collect();
        results.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        results
    }
}

// ---------------------------------------------------------------------------
// Request types used by operations
// ---------------------------------------------------------------------------

/// Parameters for creating a new skill.
#[derive(Debug)]
pub struct CreateSkillRequest {
    pub name: String,
    pub kind: SkillKind,
    pub language: Option<String>,
    pub description: Option<String>,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

/// Parameters for updating an existing skill (all fields optional).
#[derive(Debug, Default)]
pub struct UpdateSkillRequest {
    pub name: Option<String>,
    pub kind: Option<SkillKind>,
    /// `Some(None)` means "clear the field"; `None` means "leave unchanged".
    /// For simplicity we use `Option<Option<String>>` for nullable fields.
    pub language: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
}
