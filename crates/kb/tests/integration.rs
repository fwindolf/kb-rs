use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn kb() -> Command {
    Command::cargo_bin("kb").unwrap()
}

fn init_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    kb().args(["init"])
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

fn init_project_with_domain(domain: &str) -> TempDir {
    let dir = init_project();
    kb().args(["add", domain])
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

fn record_convention(dir: &TempDir, domain: &str, content: &str) {
    kb().args(["record", domain, "--type", "convention", content])
        .current_dir(dir.path())
        .assert()
        .success();
}

fn record_pattern(dir: &TempDir, domain: &str, name: &str, desc: &str) {
    kb().args([
        "record",
        domain,
        "--type",
        "pattern",
        "--name",
        name,
        "--description",
        desc,
    ])
    .current_dir(dir.path())
    .assert()
    .success();
}

fn query_json(dir: &TempDir, domain: &str) -> serde_json::Value {
    let output = kb()
        .args(["--json", "query", domain])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn query_all_json(dir: &TempDir) -> serde_json::Value {
    let output = kb()
        .args(["--json", "query", "--all"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn get_record_id(dir: &TempDir, domain: &str, index: usize) -> String {
    let json = query_json(dir, domain);
    json["domains"][0]["records"][index]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. PROJECT INITIALIZATION
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn init_creates_kb_directory() {
    let dir = TempDir::new().unwrap();
    kb().args(["init"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized"));

    assert!(dir.path().join(".kb").is_dir());
    assert!(dir.path().join(".kb/kb.config.yaml").is_file());
    assert!(dir.path().join(".kb/expertise").is_dir());
}

#[test]
fn init_is_idempotent() {
    let dir = TempDir::new().unwrap();
    kb().args(["init"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["init"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(dir.path().join(".kb/kb.config.yaml").is_file());
}

#[test]
fn init_preserves_existing_config() {
    let dir = init_project_with_domain("mydom");

    // Re-init should not lose the domain
    kb().args(["init"])
        .current_dir(dir.path())
        .assert()
        .success();

    let config = fs::read_to_string(dir.path().join(".kb/kb.config.yaml")).unwrap();
    assert!(config.contains("mydom"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. ADD DOMAIN
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn add_domain_creates_expertise_file() {
    let dir = init_project();
    kb().args(["add", "backend"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Added domain"));

    assert!(dir.path().join(".kb/expertise/backend.jsonl").is_file());

    let config = fs::read_to_string(dir.path().join(".kb/kb.config.yaml")).unwrap();
    assert!(config.contains("backend"));
}

#[test]
fn add_duplicate_domain_fails() {
    let dir = init_project_with_domain("backend");
    kb().args(["add", "backend"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn add_multiple_domains() {
    let dir = init_project();
    kb().args(["add", "frontend"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "backend"])
        .current_dir(dir.path())
        .assert()
        .success();

    let config = fs::read_to_string(dir.path().join(".kb/kb.config.yaml")).unwrap();
    assert!(config.contains("frontend"));
    assert!(config.contains("backend"));
}

#[test]
fn add_invalid_domain_name_fails() {
    let dir = init_project();
    kb().args(["add", "Invalid Name!"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2b. REMOVE DOMAIN
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn remove_empty_domain() {
    let dir = init_project_with_domain("test");
    kb().args(["remove", "test"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed domain"));

    let config = fs::read_to_string(dir.path().join(".kb/kb.config.yaml")).unwrap();
    assert!(!config.contains("test"));
    assert!(!dir.path().join(".kb/expertise/test.jsonl").exists());
}

#[test]
fn remove_nonexistent_domain_fails() {
    let dir = init_project();
    kb().args(["remove", "nope"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn remove_domain_with_records_requires_force() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Use snake_case");

    // Without --force should fail
    kb().args(["remove", "test"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("--force"));

    // With --force should succeed
    kb().args(["remove", "test", "--force"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed domain"));

    assert!(!dir.path().join(".kb/expertise/test.jsonl").exists());
}

#[test]
fn remove_domain_json_output() {
    let dir = init_project_with_domain("test");
    kb().args(["--json", "remove", "test"])
        .current_dir(dir.path())
        .assert()
        .success();
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. RECORD ALL 6 TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn record_convention_type() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "Always use snake_case for variables",
    ])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("Recorded convention"));

    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"][0]["type"], "convention");
    assert_eq!(
        json["domains"][0]["records"][0]["content"],
        "Always use snake_case for variables"
    );
}

#[test]
fn record_pattern_type() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "pattern",
        "--name",
        "Error Handling",
        "--description",
        "Use Result type for error handling",
        "--files",
        "src/error.rs,src/lib.rs",
    ])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("Recorded pattern"));

    let json = query_json(&dir, "test");
    let rec = &json["domains"][0]["records"][0];
    assert_eq!(rec["type"], "pattern");
    assert_eq!(rec["name"], "Error Handling");
    assert_eq!(rec["description"], "Use Result type for error handling");
    assert_eq!(rec["files"][0], "src/error.rs");
    assert_eq!(rec["files"][1], "src/lib.rs");
}

#[test]
fn record_failure() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "failure",
        "--description",
        "datetime.now() without timezone",
        "--resolution",
        "Always use datetime.now(UTC)",
    ])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("Recorded failure"));

    let json = query_json(&dir, "test");
    let rec = &json["domains"][0]["records"][0];
    assert_eq!(rec["type"], "failure");
    assert_eq!(rec["description"], "datetime.now() without timezone");
    assert_eq!(rec["resolution"], "Always use datetime.now(UTC)");
}

#[test]
fn record_decision() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "decision",
        "--title",
        "Use SQLite",
        "--rationale",
        "Lightweight and embedded",
    ])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("Recorded decision"));

    let json = query_json(&dir, "test");
    let rec = &json["domains"][0]["records"][0];
    assert_eq!(rec["type"], "decision");
    assert_eq!(rec["title"], "Use SQLite");
    assert_eq!(rec["rationale"], "Lightweight and embedded");
}

#[test]
fn record_reference() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "reference",
        "--name",
        "Coverage threshold",
        "--description",
        "80% minimum for new code",
        "--files",
        "pyproject.toml",
    ])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("Recorded reference"));

    let json = query_json(&dir, "test");
    let rec = &json["domains"][0]["records"][0];
    assert_eq!(rec["type"], "reference");
    assert_eq!(rec["name"], "Coverage threshold");
}

#[test]
fn record_guide() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "guide",
        "--name",
        "Adding integration tests",
        "--description",
        "Tests go in test/integration/",
    ])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("Recorded guide"));

    let json = query_json(&dir, "test");
    let rec = &json["domains"][0]["records"][0];
    assert_eq!(rec["type"], "guide");
    assert_eq!(rec["name"], "Adding integration tests");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. RECORD OPTIONS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn record_with_classification() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "--classification",
        "foundational",
        "Core convention",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let json = query_json(&dir, "test");
    assert_eq!(
        json["domains"][0]["records"][0]["classification"],
        "foundational"
    );
}

#[test]
fn record_with_tags() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "--tags",
        "safety,performance",
        "Tagged convention",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let json = query_json(&dir, "test");
    let tags = &json["domains"][0]["records"][0]["tags"];
    assert_eq!(tags[0], "safety");
    assert_eq!(tags[1], "performance");
}

#[test]
fn record_with_evidence() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "--evidence-commit",
        "abc123",
        "--evidence-issue",
        "#42",
        "Evidenced convention",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let json = query_json(&dir, "test");
    let ev = &json["domains"][0]["records"][0]["evidence"];
    assert_eq!(ev["commit"], "abc123");
    assert_eq!(ev["issue"], "#42");
}

#[test]
fn record_with_outcome() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "pattern",
        "--name",
        "Tested Pattern",
        "--description",
        "A pattern with outcome",
        "--outcome-status",
        "success",
        "--outcome-duration",
        "150",
        "--outcome-agent",
        "claude",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let json = query_json(&dir, "test");
    let outcome = &json["domains"][0]["records"][0]["outcomes"][0];
    assert_eq!(outcome["status"], "success");
    assert_eq!(outcome["duration"], serde_json::json!(150.0));
    assert_eq!(outcome["agent"], "claude");
}

#[test]
fn record_with_relates_to_and_supersedes() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "--relates-to",
        "mx-abc123",
        "--supersedes",
        "mx-old001",
        "Linked convention",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let json = query_json(&dir, "test");
    let rec = &json["domains"][0]["records"][0];
    assert_eq!(rec["relates_to"][0], "mx-abc123");
    assert_eq!(rec["supersedes"][0], "mx-old001");
}

#[test]
fn record_missing_required_fields_fails() {
    let dir = init_project_with_domain("test");

    // Pattern without --name
    kb().args([
        "record",
        "test",
        "--type",
        "pattern",
        "--description",
        "desc",
    ])
    .current_dir(dir.path())
    .assert()
    .failure();

    // Failure without --resolution
    kb().args([
        "record",
        "test",
        "--type",
        "failure",
        "--description",
        "broken",
    ])
    .current_dir(dir.path())
    .assert()
    .failure();

    // Decision without --rationale
    kb().args(["record", "test", "--type", "decision", "--title", "title"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn record_unknown_type_fails() {
    let dir = init_project_with_domain("test");
    kb().args(["record", "test", "--type", "invalid", "content"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. RECORD DEDUPLICATION
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn duplicate_convention_is_skipped() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Use snake_case");
    // Same content again
    kb().args(["record", "test", "--type", "convention", "Use snake_case"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Duplicate"));

    let json = query_json(&dir, "test");
    // Only one record
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 1);
}

#[test]
fn duplicate_pattern_is_upserted() {
    let dir = init_project_with_domain("test");
    record_pattern(&dir, "test", "MyPattern", "Version 1");
    // Same name, new description → upsert
    record_pattern(&dir, "test", "MyPattern", "Version 2");

    let json = query_json(&dir, "test");
    let records = json["domains"][0]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["description"], "Version 2");
}

#[test]
fn force_flag_bypasses_dedup() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "same content");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "--force",
        "same content",
    ])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("Recorded"));

    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. DRY-RUN
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn record_dry_run_does_not_write() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "--dry-run",
        "Dry run content",
    ])
    .current_dir(dir.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("Dry-run"));

    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7. STDIN AND BATCH RECORDING
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn record_from_stdin() {
    let dir = init_project_with_domain("test");
    let input = r#"[
        {"type":"convention","content":"From stdin","classification":"tactical"},
        {"type":"pattern","name":"StdinPattern","description":"Also from stdin","classification":"tactical"}
    ]"#;

    kb().args(["record", "test", "--stdin"])
        .write_stdin(input)
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created 2"));

    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 2);
}

#[test]
fn record_from_batch_file() {
    let dir = init_project_with_domain("test");
    let batch_path = dir.path().join("batch.json");
    fs::write(
        &batch_path,
        r#"[
        {"type":"convention","content":"Batch convention","classification":"tactical"},
        {"type":"decision","title":"Batch decision","rationale":"Batch rationale","classification":"tactical"}
    ]"#,
    )
    .unwrap();

    kb().args(["record", "test", "--batch", batch_path.to_str().unwrap()])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created 2"));

    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8. EDIT
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn edit_convention_content() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Original content");
    let id = get_record_id(&dir, "test", 0);

    kb().args(["edit", "test", &id, "--content", "Updated content"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));

    let json = query_json(&dir, "test");
    assert_eq!(
        json["domains"][0]["records"][0]["content"],
        "Updated content"
    );
}

#[test]
fn edit_pattern_fields() {
    let dir = init_project_with_domain("test");
    record_pattern(&dir, "test", "OldName", "Old description");
    let id = get_record_id(&dir, "test", 0);

    kb().args([
        "edit",
        "test",
        &id,
        "--name",
        "NewName",
        "--description",
        "New description",
        "--files",
        "src/new.rs",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let json = query_json(&dir, "test");
    let rec = &json["domains"][0]["records"][0];
    assert_eq!(rec["name"], "NewName");
    assert_eq!(rec["description"], "New description");
    assert_eq!(rec["files"][0], "src/new.rs");
}

#[test]
fn edit_classification() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "A convention");
    let id = get_record_id(&dir, "test", 0);

    kb().args(["edit", "test", &id, "--classification", "foundational"])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    assert_eq!(
        json["domains"][0]["records"][0]["classification"],
        "foundational"
    );
}

#[test]
fn edit_append_outcome() {
    let dir = init_project_with_domain("test");
    record_pattern(&dir, "test", "TestPattern", "A pattern");
    let id = get_record_id(&dir, "test", 0);

    kb().args([
        "edit",
        "test",
        &id,
        "--outcome-status",
        "success",
        "--outcome-agent",
        "claude",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let json = query_json(&dir, "test");
    let outcomes = &json["domains"][0]["records"][0]["outcomes"];
    assert_eq!(outcomes.as_array().unwrap().len(), 1);
    assert_eq!(outcomes[0]["status"], "success");

    // Append another outcome
    kb().args(["edit", "test", &id, "--outcome-status", "failure"])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    let outcomes = &json["domains"][0]["records"][0]["outcomes"];
    assert_eq!(outcomes.as_array().unwrap().len(), 2);
    assert_eq!(outcomes[1]["status"], "failure");
}

#[test]
fn edit_nonexistent_record_fails() {
    let dir = init_project_with_domain("test");
    kb().args(["edit", "test", "mx-nonexistent", "--content", "nope"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9. DELETE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn delete_record_by_id() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "To be deleted");
    record_convention(&dir, "test", "To be kept");
    let id = get_record_id(&dir, "test", 0);

    kb().args(["delete", "test", &id])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));

    let json = query_json(&dir, "test");
    let records = json["domains"][0]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["content"], "To be kept");
}

#[test]
fn delete_preserves_other_records() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "First");
    record_convention(&dir, "test", "Second");
    record_convention(&dir, "test", "Third");

    let id = get_record_id(&dir, "test", 1); // middle record

    kb().args(["delete", "test", &id])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    let records = json["domains"][0]["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["content"], "First");
    assert_eq!(records[1]["content"], "Third");
}

#[test]
fn delete_last_record_leaves_empty() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Only record");
    let id = get_record_id(&dir, "test", 0);

    kb().args(["delete", "test", &id])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 0);
}

#[test]
fn delete_nonexistent_record_fails() {
    let dir = init_project_with_domain("test");
    kb().args(["delete", "test", "mx-nonexistent"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10. QUERY
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn query_single_domain() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "A convention");
    record_pattern(&dir, "test", "APattern", "A pattern");

    kb().args(["query", "test"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("## test"))
        .stdout(predicate::str::contains("Conventions"))
        .stdout(predicate::str::contains("Patterns"));
}

#[test]
fn query_all_domains() {
    let dir = init_project();
    kb().args(["add", "dom1"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "dom2"])
        .current_dir(dir.path())
        .assert()
        .success();
    record_convention(&dir, "dom1", "Convention 1");
    record_convention(&dir, "dom2", "Convention 2");

    kb().args(["query", "--all"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("## dom1"))
        .stdout(predicate::str::contains("## dom2"));
}

#[test]
fn query_filter_by_type() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "A convention");
    record_pattern(&dir, "test", "APattern", "A pattern");

    let json = {
        let output = kb()
            .args(["--json", "query", "test", "--type", "pattern"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        v
    };

    let records = json["domains"][0]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["type"], "pattern");
}

#[test]
fn query_filter_by_classification() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "--classification",
        "foundational",
        "Foundational stuff",
    ])
    .current_dir(dir.path())
    .assert()
    .success();
    record_convention(&dir, "test", "Tactical stuff");

    let json = {
        let output = kb()
            .args([
                "--json",
                "query",
                "test",
                "--classification",
                "foundational",
            ])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        v
    };

    let records = json["domains"][0]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["content"], "Foundational stuff");
}

#[test]
fn query_nonexistent_domain_fails() {
    let dir = init_project();
    kb().args(["query", "nonexistent"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn query_without_domain_and_no_all_fails() {
    let dir = init_project();
    kb().args(["add", "dom1"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "dom2"])
        .current_dir(dir.path())
        .assert()
        .success();

    kb().args(["query"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11. SEARCH
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn search_finds_matching_records() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Always use error handling");
    record_pattern(&dir, "test", "Error Pattern", "Handle errors with Result");
    record_convention(&dir, "test", "Unrelated naming convention");

    kb().args(["search", "error handling"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("error"))
        .stdout(predicate::str::contains("match"));
}

#[test]
fn search_no_results() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Use snake_case");

    kb().args(["search", "nonexistent_xyz_query"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No records matching"));
}

#[test]
fn search_across_domains() {
    let dir = init_project();
    kb().args(["add", "dom1"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "dom2"])
        .current_dir(dir.path())
        .assert()
        .success();
    record_convention(&dir, "dom1", "Error handling in domain 1");
    record_convention(&dir, "dom2", "Error handling in domain 2");

    let json = {
        let output = kb()
            .args(["--json", "search", "error handling"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        v
    };

    assert!(json["total"].as_u64().unwrap() >= 2);
}

#[test]
fn search_with_domain_filter() {
    let dir = init_project();
    kb().args(["add", "dom1"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "dom2"])
        .current_dir(dir.path())
        .assert()
        .success();
    record_convention(&dir, "dom1", "Error in dom1");
    record_convention(&dir, "dom2", "Error in dom2");

    let json = {
        let output = kb()
            .args(["--json", "search", "error", "--domain", "dom1"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        v
    };

    let domains = json["domains"].as_array().unwrap();
    assert_eq!(domains.len(), 1);
    assert_eq!(domains[0]["domain"], "dom1");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12. STATUS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn status_shows_domain_info() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "A convention");

    kb().args(["status"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("KB Status"))
        .stdout(predicate::str::contains("test:"))
        .stdout(predicate::str::contains("1 records"));
}

#[test]
fn status_json_output() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "A convention");

    let output = kb()
        .args(["--json", "status"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["command"], "status");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 13. VALIDATE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn validate_clean_data() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Valid convention");
    record_pattern(&dir, "test", "Valid", "Valid pattern");

    kb().args(["validate"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "2 records validated, 0 errors found",
        ));
}

#[test]
fn validate_corrupt_data() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Valid convention");

    // Append corrupt line
    let file_path = dir.path().join(".kb/expertise/test.jsonl");
    let mut content = fs::read_to_string(&file_path).unwrap();
    content.push_str("{\"type\":\"invalid_garbage\"}\n");
    fs::write(&file_path, content).unwrap();

    // Error output goes to stderr (colored)
    kb().args(["validate"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("errors found"));
}

#[test]
fn validate_json_output() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "A convention");

    let output = kb()
        .args(["--json", "validate"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["valid"], true);
    assert_eq!(json["totalRecords"], 1);
    assert_eq!(json["totalErrors"], 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 14. DOCTOR
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn doctor_healthy_project() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "A convention");

    kb().args(["doctor"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));
}

#[test]
fn doctor_detects_corrupt_data() {
    let dir = init_project_with_domain("test");
    let file_path = dir.path().join(".kb/expertise/test.jsonl");
    fs::write(&file_path, "not valid json\n").unwrap();

    kb().args(["doctor"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("issue"));
}

#[test]
fn doctor_fix_removes_bad_lines() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Good convention");

    let file_path = dir.path().join(".kb/expertise/test.jsonl");
    let mut content = fs::read_to_string(&file_path).unwrap();
    content.push_str("bad json line\n");
    fs::write(&file_path, content).unwrap();

    kb().args(["doctor", "--fix"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Fixed"));

    // After fix, should be clean
    kb().args(["doctor"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));
}

#[test]
fn doctor_detects_orphan_files() {
    let dir = init_project_with_domain("test");

    // Create orphan file
    fs::write(dir.path().join(".kb/expertise/orphan.jsonl"), "").unwrap();

    kb().args(["doctor"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Orphan"));
}

#[test]
fn doctor_json_output() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "A convention");

    let output = kb()
        .args(["--json", "doctor"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["command"], "doctor");
    assert!(json["issues"].as_array().unwrap().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 15. PRIME
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn prime_markdown_output() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Prime convention");

    kb().args(["prime"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Project Expertise"))
        .stdout(predicate::str::contains("Prime convention"))
        .stdout(predicate::str::contains("SESSION CLOSE PROTOCOL"));
}

#[test]
fn prime_xml_output() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "XML convention");

    kb().args(["prime", "--format", "xml"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("<expertise>"))
        .stdout(predicate::str::contains("</expertise>"))
        .stdout(predicate::str::contains("XML convention"));
}

#[test]
fn prime_plain_output() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Plain convention");

    kb().args(["prime", "--format", "plain"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Expertise"))
        .stdout(predicate::str::contains("===="))
        .stdout(predicate::str::contains("Plain convention"));
}

#[test]
fn prime_mcp_json_output() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "MCP convention");

    let output = kb()
        .args(["prime", "--mcp"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "expertise");
    assert!(json["domains"].is_array());
}

#[test]
fn prime_domain_scoping() {
    let dir = init_project();
    kb().args(["add", "dom1"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "dom2"])
        .current_dir(dir.path())
        .assert()
        .success();
    record_convention(&dir, "dom1", "Dom1 convention");
    record_convention(&dir, "dom2", "Dom2 convention");

    kb().args(["prime", "dom1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dom1 convention"))
        .stdout(predicate::str::contains("Dom2 convention").not());
}

#[test]
fn prime_domain_exclusion() {
    let dir = init_project();
    kb().args(["add", "dom1"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "dom2"])
        .current_dir(dir.path())
        .assert()
        .success();
    record_convention(&dir, "dom1", "Dom1 convention");
    record_convention(&dir, "dom2", "Dom2 convention");

    kb().args(["prime", "--exclude-domain", "dom2"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dom1 convention"))
        .stdout(predicate::str::contains("Dom2 convention").not());
}

#[test]
fn prime_export_to_file() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Export convention");
    let export_path = dir.path().join("output.md");

    kb().args(["prime", "--export", export_path.to_str().unwrap()])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Exported"));

    let content = fs::read_to_string(&export_path).unwrap();
    assert!(content.contains("Export convention"));
}

#[test]
fn prime_full_shows_classification() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "convention",
        "--classification",
        "foundational",
        "Full convention",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    kb().args(["prime", "--full"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("foundational"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 16. PRUNE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn prune_no_stale_records() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Fresh convention");

    kb().args(["prune"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale records"));
}

#[test]
fn prune_removes_stale_tactical_records() {
    let dir = init_project_with_domain("test");

    // Write a stale tactical record directly (15 days old)
    let file_path = dir.path().join(".kb/expertise/test.jsonl");
    let old_date = (chrono::Utc::now() - chrono::Duration::days(15))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let record = format!(
        r#"{{"type":"convention","content":"Old stale content","classification":"tactical","recorded_at":"{}","id":"mx-stale1"}}"#,
        old_date
    );
    fs::write(&file_path, format!("{record}\n")).unwrap();

    // Also add a fresh record
    record_convention(&dir, "test", "Fresh content");

    kb().args(["prune"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("pruned 1 stale record"));

    let json = query_json(&dir, "test");
    let records = json["domains"][0]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["content"], "Fresh content");
}

#[test]
fn prune_never_removes_foundational() {
    let dir = init_project_with_domain("test");

    // Write a foundational record that's very old
    let file_path = dir.path().join(".kb/expertise/test.jsonl");
    let old_date = (chrono::Utc::now() - chrono::Duration::days(365))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let record = format!(
        r#"{{"type":"convention","content":"Foundational content","classification":"foundational","recorded_at":"{}","id":"mx-found1"}}"#,
        old_date
    );
    fs::write(&file_path, format!("{record}\n")).unwrap();

    kb().args(["prune"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No stale records"));
}

#[test]
fn prune_dry_run() {
    let dir = init_project_with_domain("test");

    let file_path = dir.path().join(".kb/expertise/test.jsonl");
    let old_date = (chrono::Utc::now() - chrono::Duration::days(15))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let record = format!(
        r#"{{"type":"convention","content":"Stale content","classification":"tactical","recorded_at":"{}","id":"mx-stale2"}}"#,
        old_date
    );
    fs::write(&file_path, format!("{record}\n")).unwrap();

    kb().args(["prune", "--dry-run"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("would be pruned"));

    // Record should still exist
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("Stale content"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 17. READY
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ready_shows_recent_records() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Recent convention");
    record_pattern(&dir, "test", "Recent", "Recent pattern");

    kb().args(["ready"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Recent records"))
        .stdout(predicate::str::contains("Recent convention"))
        .stdout(predicate::str::contains("Recent"));
}

#[test]
fn ready_with_limit() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "First");
    record_convention(&dir, "test", "Second");
    record_convention(&dir, "test", "Third");

    let output = kb()
        .args(["--json", "ready", "--limit", "2"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["count"], 2);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 18. JSON OUTPUT MODE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn json_query_structure() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "JSON test convention");
    record_pattern(&dir, "test", "JSONPat", "JSON test pattern");

    let json = query_json(&dir, "test");
    assert_eq!(json["success"], true);
    assert_eq!(json["command"], "query");
    assert!(json["domains"].is_array());
    assert_eq!(json["domains"][0]["domain"], "test");
    assert!(json["domains"][0]["records"].is_array());
}

#[test]
fn json_search_structure() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Searchable content");

    let output = kb()
        .args(["--json", "search", "searchable"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["command"], "search");
    assert!(json["total"].is_number());
    assert!(json["domains"].is_array());
}

#[test]
fn json_record_created() {
    let dir = init_project_with_domain("test");

    let output = kb()
        .args([
            "--json",
            "record",
            "test",
            "--type",
            "convention",
            "JSON record",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["action"], "created");
    assert_eq!(json["domain"], "test");
    assert!(json["record"]["id"].is_string());
}

#[test]
fn json_delete_structure() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "To delete");
    let id = get_record_id(&dir, "test", 0);

    let output = kb()
        .args(["--json", "delete", "test", &id])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["command"], "delete");
    assert_eq!(json["id"], id);
}

#[test]
fn json_edit_structure() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "To edit");
    let id = get_record_id(&dir, "test", 0);

    let output = kb()
        .args(["--json", "edit", "test", &id, "--content", "Edited"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["command"], "edit");
    assert_eq!(json["record"]["content"], "Edited");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 19. ERROR HANDLING
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn operations_without_init_fail() {
    let dir = TempDir::new().unwrap();

    kb().args(["query", "--all"])
        .current_dir(dir.path())
        .assert()
        .failure();

    kb().args(["status"])
        .current_dir(dir.path())
        .assert()
        .failure();

    kb().args(["validate"])
        .current_dir(dir.path())
        .assert()
        .failure();

    kb().args(["prime"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn record_to_missing_domain_fails() {
    let dir = init_project();
    kb().args(["record", "nonexistent", "--type", "convention", "content"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn json_error_output() {
    let dir = TempDir::new().unwrap();
    let output = kb()
        .args(["--json", "status"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should still produce valid JSON even on error
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["success"], false);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 20. FULL CRUD LIFECYCLE
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn full_crud_lifecycle() {
    let dir = init_project_with_domain("test");

    // Create
    record_convention(&dir, "test", "Original content");
    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 1);
    let id = get_record_id(&dir, "test", 0);

    // Read (query)
    kb().args(["query", "test"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Original content"));

    // Update (edit)
    kb().args(["edit", "test", &id, "--content", "Updated content"])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    assert_eq!(
        json["domains"][0]["records"][0]["content"],
        "Updated content"
    );

    // Delete
    kb().args(["delete", "test", &id])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 21. MULTI-DOMAIN WORKFLOW
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn multi_domain_workflow() {
    let dir = init_project();

    // Add multiple domains
    kb().args(["add", "frontend"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "backend"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["add", "infra"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Record in different domains
    record_convention(&dir, "frontend", "Use React hooks");
    record_pattern(
        &dir,
        "backend",
        "API Pattern",
        "RESTful endpoints with validation",
    );
    kb().args([
        "record",
        "infra",
        "--type",
        "decision",
        "--title",
        "Use Docker",
        "--rationale",
        "Container isolation",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    // Query all
    let json = query_all_json(&dir);
    let domains = json["domains"].as_array().unwrap();
    assert_eq!(domains.len(), 3);

    // Status shows all
    kb().args(["status"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("frontend"))
        .stdout(predicate::str::contains("backend"))
        .stdout(predicate::str::contains("infra"));

    // Validate all
    kb().args(["validate"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "3 records validated, 0 errors found",
        ));

    // Search across all domains
    kb().args(["search", "pattern"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Prime shows all domains
    kb().args(["prime"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("frontend"))
        .stdout(predicate::str::contains("backend"))
        .stdout(predicate::str::contains("infra"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 22. RECORD ID RESOLUTION
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn resolve_by_prefix() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Prefix resolve test");
    let full_id = get_record_id(&dir, "test", 0); // e.g. "mx-abc123"
    let prefix = &full_id[3..6]; // first 3 chars of hash

    kb().args(["edit", "test", prefix, "--content", "Resolved by prefix"])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    assert_eq!(
        json["domains"][0]["records"][0]["content"],
        "Resolved by prefix"
    );
}

#[test]
fn resolve_by_bare_hash() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Hash resolve test");
    let full_id = get_record_id(&dir, "test", 0); // e.g. "mx-abc123"
    let hash = &full_id[3..]; // strip "mx-"

    kb().args(["edit", "test", hash, "--content", "Resolved by hash"])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    assert_eq!(
        json["domains"][0]["records"][0]["content"],
        "Resolved by hash"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 23. RECORD ID GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn records_get_unique_ids() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "First convention");
    record_convention(&dir, "test", "Second convention");
    record_pattern(&dir, "test", "A Pattern", "Description");

    let json = query_json(&dir, "test");
    let records = json["domains"][0]["records"].as_array().unwrap();
    let ids: Vec<&str> = records.iter().map(|r| r["id"].as_str().unwrap()).collect();

    // All IDs start with "mx-"
    for id in &ids {
        assert!(id.starts_with("mx-"), "ID should start with mx-: {}", id);
    }

    // All IDs are unique
    let unique_ids: std::collections::HashSet<&&str> = ids.iter().collect();
    assert_eq!(unique_ids.len(), ids.len());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 24. ALL RECORD TYPES IN ONE DOMAIN (comprehensive formatting)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn all_record_types_query_formatting() {
    let dir = init_project_with_domain("test");

    record_convention(&dir, "test", "A convention");
    record_pattern(&dir, "test", "MyPattern", "Pattern description");
    kb().args([
        "record",
        "test",
        "--type",
        "failure",
        "--description",
        "A failure",
        "--resolution",
        "The fix",
    ])
    .current_dir(dir.path())
    .assert()
    .success();
    kb().args([
        "record",
        "test",
        "--type",
        "decision",
        "--title",
        "My Decision",
        "--rationale",
        "Good reasons",
    ])
    .current_dir(dir.path())
    .assert()
    .success();
    kb().args([
        "record",
        "test",
        "--type",
        "reference",
        "--name",
        "MyRef",
        "--description",
        "Reference desc",
    ])
    .current_dir(dir.path())
    .assert()
    .success();
    kb().args([
        "record",
        "test",
        "--type",
        "guide",
        "--name",
        "MyGuide",
        "--description",
        "Guide desc",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    // Query should show all sections
    kb().args(["query", "test"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("### Conventions"))
        .stdout(predicate::str::contains("### Patterns"))
        .stdout(predicate::str::contains("### Known Failures"))
        .stdout(predicate::str::contains("### Decisions"))
        .stdout(predicate::str::contains("### References"))
        .stdout(predicate::str::contains("### Guides"));

    // Query --all JSON should have 6 records
    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"].as_array().unwrap().len(), 6);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 25. QUERY WITH FILE FILTER
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn query_filter_by_file() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "pattern",
        "--name",
        "Relevant",
        "--description",
        "Relevant pattern",
        "--files",
        "src/main.rs",
    ])
    .current_dir(dir.path())
    .assert()
    .success();
    record_convention(&dir, "test", "No files attached");

    let json = {
        let output = kb()
            .args(["--json", "query", "test", "--file", "main.rs"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        v
    };

    let records = json["domains"][0]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["name"], "Relevant");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 26. QUERY WITH OUTCOME STATUS FILTER
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn query_filter_by_outcome_status() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "pattern",
        "--name",
        "Success",
        "--description",
        "Succeeded",
        "--outcome-status",
        "success",
    ])
    .current_dir(dir.path())
    .assert()
    .success();
    kb().args([
        "record",
        "test",
        "--type",
        "pattern",
        "--name",
        "Failure",
        "--description",
        "Failed",
        "--outcome-status",
        "failure",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let json = {
        let output = kb()
            .args(["--json", "query", "test", "--outcome-status", "success"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        v
    };

    let records = json["domains"][0]["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["name"], "Success");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 27. EDIT FAILURE AND DECISION RECORDS
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn edit_failure_resolution() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "failure",
        "--description",
        "A bug",
        "--resolution",
        "Old fix",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let id = get_record_id(&dir, "test", 0);
    kb().args(["edit", "test", &id, "--resolution", "Better fix"])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    assert_eq!(json["domains"][0]["records"][0]["resolution"], "Better fix");
}

#[test]
fn edit_decision_rationale() {
    let dir = init_project_with_domain("test");
    kb().args([
        "record",
        "test",
        "--type",
        "decision",
        "--title",
        "Use Postgres",
        "--rationale",
        "Old rationale",
    ])
    .current_dir(dir.path())
    .assert()
    .success();

    let id = get_record_id(&dir, "test", 0);
    kb().args(["edit", "test", &id, "--rationale", "Better rationale"])
        .current_dir(dir.path())
        .assert()
        .success();

    let json = query_json(&dir, "test");
    assert_eq!(
        json["domains"][0]["records"][0]["rationale"],
        "Better rationale"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 28. RECORD TIMESTAMP
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn records_have_valid_timestamps() {
    let dir = init_project_with_domain("test");
    record_convention(&dir, "test", "Timestamped");

    let json = query_json(&dir, "test");
    let recorded_at = json["domains"][0]["records"][0]["recorded_at"]
        .as_str()
        .unwrap();

    // Should be a valid RFC 3339 timestamp
    assert!(
        chrono::DateTime::parse_from_rfc3339(recorded_at).is_ok(),
        "recorded_at should be valid RFC 3339: {}",
        recorded_at
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 29. UPDATE COMMAND
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn update_command_shows_info() {
    let dir = init_project();
    kb().args(["update"])
        .current_dir(dir.path())
        .assert()
        .success();
}

// ═══════════════════════════════════════════════════════════════════════════════
// 30. SINGLE DOMAIN AUTO-SELECT
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn query_auto_selects_single_domain() {
    let dir = init_project_with_domain("only");
    record_convention(&dir, "only", "Single domain");

    // Should work without specifying domain or --all when there's only one
    kb().args(["query"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Single domain"));
}

// ── Onboard ─────────────────────────────────────────────────────────────────

#[test]
fn onboard_creates_agents_md() {
    let dir = init_project();
    kb().args(["onboard"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote onboarding to"));

    let content = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("<!-- kb:start -->"));
    assert!(content.contains("<!-- kb:end -->"));
    assert!(content.contains("kb prime"));
}

#[test]
fn onboard_prefers_claude_md() {
    let dir = init_project();
    fs::write(dir.path().join("CLAUDE.md"), "# Project\n").unwrap();

    kb().args(["onboard"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("CLAUDE.md"));

    // AGENTS.md should NOT be created
    assert!(!dir.path().join("AGENTS.md").exists());
}

#[test]
fn onboard_check_reports_installed() {
    let dir = init_project();
    // Install first
    kb().args(["onboard"])
        .current_dir(dir.path())
        .assert()
        .success();
    // Check
    kb().args(["onboard", "--check"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("installed"));
}

#[test]
fn onboard_check_reports_missing() {
    let dir = init_project();
    fs::write(dir.path().join("AGENTS.md"), "# No kb\n").unwrap();

    kb().args(["onboard", "--check"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn onboard_remove_deletes_section() {
    let dir = init_project();
    // Install
    kb().args(["onboard"])
        .current_dir(dir.path())
        .assert()
        .success();
    assert!(dir.path().join("AGENTS.md").exists());

    // Remove — file was only the onboard section, should be deleted
    kb().args(["onboard", "--remove"])
        .current_dir(dir.path())
        .assert()
        .success();
    assert!(!dir.path().join("AGENTS.md").exists());
}

#[test]
fn onboard_remove_preserves_other_content() {
    let dir = init_project();
    // Create file with extra content, then onboard
    fs::write(dir.path().join("AGENTS.md"), "# My Project\n").unwrap();
    kb().args(["onboard"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Remove
    kb().args(["onboard", "--remove"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed onboard section from"));

    // File should still exist with original content
    let content = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("# My Project"));
    assert!(!content.contains("kb:start"));
}

#[test]
fn onboard_remove_noop_no_file() {
    let dir = init_project();
    // No AGENTS.md or CLAUDE.md — should be a noop
    kb().args(["onboard", "--remove"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No AGENTS.md found"));
}

#[test]
fn onboard_is_idempotent() {
    let dir = init_project();
    // Install twice
    kb().args(["onboard"])
        .current_dir(dir.path())
        .assert()
        .success();
    kb().args(["onboard"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already exists"));
}
