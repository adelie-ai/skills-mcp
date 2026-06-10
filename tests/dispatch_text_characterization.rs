#![deny(warnings)]

// MF-15 characterization tests (CODE_REVIEW_2026-06-09.md §8.3).
//
// The operations historically returned the pre-migration
// `{"content":[{"type":"text","text":...}]}` envelope which
// `service::dispatch` immediately unpacked back into a string. These tests
// pin the EXACT `ToolReply` text every tool produces — for the happy path of
// all six tools plus the error classification — so the envelope-removal
// refactor can be shown to be behavior-preserving: they must pass unchanged
// before and after.
//
// The success text is whatever `serde_json::to_string_pretty` produces for the
// typed value each operation returns (a `SkillDetail`, a `Vec<view>`, or a
// fixed sentence for delete). We assert against that same typed value read back
// from disk, which both pins the exact bytes and keeps struct field order
// (note: `serde_json::Value` round-tripping would re-sort keys and is NOT a
// valid expectation here).
//
// Everything runs inside ONE test function: the skill roots are configured
// via process-global environment variables (`$HOME`, `$SKILLS_MCP_ROOTS`,
// `$SKILLS_MCP_WRITE_ROOT`), so concurrent test threads must not race them.

use mcp_core::{CallError, Content, McpService, ToolReply};
use serde_json::{Value, json};
use skills_mcp::repo;
use skills_mcp::service::SkillsService;

/// Extract the single text block from a successful reply.
fn reply_text(reply: &ToolReply) -> String {
    assert!(!reply.is_error, "reply must not be an error");
    assert_eq!(reply.content.len(), 1, "reply must have exactly one block");
    match &reply.content[0] {
        Content::Text(t) => t.clone(),
        _ => panic!("expected a text content block"),
    }
}

#[tokio::test]
async fn dispatch_text_is_characterized_for_every_tool() {
    // -- Hermetic root setup. HOME is pointed at the temp dir so the real
    // ~/.agents/skills and ~/.claude/skills can never leak into list/search.
    let temp = tempdir();
    let root = temp.path.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let root_str = root.display().to_string();
    unsafe {
        std::env::set_var("HOME", temp.path.display().to_string());
        std::env::set_var(repo::ROOTS_ENV, &root_str);
        std::env::set_var(repo::WRITE_ROOT_ENV, &root_str);
    }

    let svc = SkillsService;

    // -- skills_create_skill: text is the pretty-printed `SkillDetail` that
    // `write_new` returns. Note this is NOT identical to a later `read` of the
    // same skill: `write_new` echoes the body verbatim, while a read parses it
    // back from rendered SKILL.md (which gains a trailing newline). So the
    // expectation is built from the exact input field values.
    let create_args = json!({
        "name": "alpha",
        "description": "Alpha test skill",
        "tags": ["demo"],
        "content": "# Alpha\n\nBody text.",
    });
    let create_text = reply_text(
        &svc.call_tool("skills_create_skill", &create_args)
            .await
            .expect("create alpha"),
    );
    let v: Value = serde_json::from_str(&create_text).expect("create reply is JSON");
    assert_eq!(v["name"], "alpha");
    assert_eq!(v["description"], "Alpha test skill");
    assert_eq!(v["tags"], json!(["demo"]));
    assert_eq!(v["content"], "# Alpha\n\nBody text.");
    assert_eq!(
        v["path"],
        root.join("alpha").join("SKILL.md").display().to_string()
    );
    assert_eq!(v["root"], root_str);
    assert_eq!(v["attachments"], json!([]));
    // Pin the exact bytes: `write_new`'s `SkillDetail` in struct field order.
    let expected_created = repo::SkillDetail {
        name: "alpha".into(),
        description: "Alpha test skill".into(),
        tags: vec!["demo".into()],
        content: "# Alpha\n\nBody text.".into(),
        path: root.join("alpha").join("SKILL.md").display().to_string(),
        root: root_str.clone(),
        attachments: vec![],
    };
    assert_eq!(
        create_text,
        serde_json::to_string_pretty(&expected_created).unwrap()
    );

    // -- skills_get_skill: text is exactly the pretty-printed detail read
    // from disk (same shape and formatting as create).
    let get_text = reply_text(
        &svc.call_tool("skills_get_skill", &json!({"name": "alpha"}))
            .await
            .expect("get alpha"),
    );
    let expected_detail = repo::read("alpha").expect("alpha exists on disk");
    assert_eq!(
        get_text,
        serde_json::to_string_pretty(&expected_detail).unwrap()
    );

    // -- skills_update_skill: pretty-printed detail with the patch applied.
    let update_text = reply_text(
        &svc.call_tool(
            "skills_update_skill",
            &json!({"name": "alpha", "description": "Updated description"}),
        )
        .await
        .expect("update alpha"),
    );
    let updated_detail = repo::read("alpha").expect("alpha still exists after update");
    assert_eq!(updated_detail.description, "Updated description");
    assert_eq!(
        update_text,
        serde_json::to_string_pretty(&updated_detail).unwrap()
    );

    // -- skills_list_skills: pretty-printed array of `to_view` projections.
    let list_text = reply_text(
        &svc.call_tool("skills_list_skills", &json!({}))
            .await
            .expect("list"),
    );
    let summaries = repo::list_all();
    assert_eq!(summaries.len(), 1, "exactly the one skill we created");
    let expected_views: Vec<Value> = summaries.iter().map(|s| s.to_view(false)).collect();
    assert_eq!(
        list_text,
        serde_json::to_string_pretty(&expected_views).unwrap()
    );

    // -- skills_search_skills: same projection, filtered by the query.
    let search_text = reply_text(
        &svc.call_tool("skills_search_skills", &json!({"query": "alpha"}))
            .await
            .expect("search"),
    );
    let expected_views: Vec<Value> = repo::search("alpha", &[])
        .iter()
        .map(|s| s.to_view(false))
        .collect();
    assert_eq!(
        search_text,
        serde_json::to_string_pretty(&expected_views).unwrap()
    );

    // -- skills_delete_skill: a fixed human-readable sentence, not JSON.
    let deleted_root = summaries[0].root.clone();
    let delete_text = reply_text(
        &svc.call_tool("skills_delete_skill", &json!({"name": "alpha"}))
            .await
            .expect("delete alpha"),
    );
    assert_eq!(
        delete_text,
        format!("Deleted skill 'alpha' from {deleted_root}")
    );

    // -- Error classification must survive the refactor too.
    // Unknown tool → CallError::Tool naming the tool.
    match svc.call_tool("skills_no_such_tool", &json!({})).await {
        Err(CallError::Tool(msg)) => assert!(msg.contains("skills_no_such_tool")),
        _ => panic!("unknown tool must be CallError::Tool"),
    }
    // Structurally invalid params (missing required field) → InvalidParams.
    match svc.call_tool("skills_get_skill", &json!({})).await {
        Err(CallError::InvalidParams(_)) => {}
        _ => panic!("missing `name` must be CallError::InvalidParams"),
    }
    // Domain-invalid input (empty content) → InvalidParams.
    match svc
        .call_tool(
            "skills_create_skill",
            &json!({"name": "beta", "description": "d", "content": "   "}),
        )
        .await
    {
        Err(CallError::InvalidParams(_)) => {}
        _ => panic!("empty content must be CallError::InvalidParams"),
    }
    // NotFound → CallError::Tool (isError content the model can react to).
    match svc
        .call_tool("skills_get_skill", &json!({"name": "alpha"}))
        .await
    {
        Err(CallError::Tool(msg)) => assert!(msg.contains("alpha")),
        _ => panic!("deleted skill lookup must be CallError::Tool"),
    }
}

/// Tiny in-tree temp-dir helper (mirrors src/repo.rs's test helper) to avoid
/// adding a tempfile dev-dependency for one test.
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
        .unwrap()
        .subsec_nanos();
    let path =
        std::env::temp_dir().join(format!("skills-mcp-charact-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    TempDir { path }
}
