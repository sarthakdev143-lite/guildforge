//! Cross-crate integration tests for the config layer.
//!
//! These tests exercise the full parse → validate pipeline against
//! every example in `examples/` and `examples/broken/`. They use the
//! real binary via `assert_cmd` to verify end-to-end behavior.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

/// Path to the workspace root (apps/cli/../..).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Path to an example file relative to the workspace root.
fn example(rel: &str) -> PathBuf {
    workspace_root().join(rel)
}

fn cmd() -> Command {
    let mut cmd = Command::cargo_bin("guildforge").unwrap();
    cmd.env("GUILDFORGE_NO_NETWORK", "1");
    cmd
}

// ===========================================================================
// Positive: every valid example exits 0
// ===========================================================================

#[test]
fn company_yaml_validates() {
    cmd()
        .args([
            "validate",
            &example("examples/company.yaml").to_string_lossy(),
        ])
        .assert()
        .success();
}

#[test]
fn community_yaml_validates() {
    cmd()
        .args([
            "validate",
            &example("examples/community.yaml").to_string_lossy(),
        ])
        .assert()
        .success();
}

#[test]
fn minimal_template_validates() {
    cmd()
        .args([
            "validate",
            &example("templates/minimal.yaml").to_string_lossy(),
        ])
        .assert()
        .success();
}

// ===========================================================================
// Negative: every broken example exits non-zero with the expected code
// ===========================================================================

#[test]
fn broken_unknown_field_exits_3() {
    cmd()
        .args([
            "validate",
            &example("examples/broken/unknown_field.yaml").to_string_lossy(),
        ])
        .assert()
        .failure()
        .code(3);
}

#[test]
fn broken_duplicate_role_exits_1_with_v001() {
    cmd()
        .args([
            "validate",
            &example("examples/broken/duplicate_role.yaml").to_string_lossy(),
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("V001"));
}

#[test]
fn broken_unknown_category_ref_exits_1_with_v010() {
    cmd()
        .args([
            "validate",
            &example("examples/broken/unknown_category_ref.yaml").to_string_lossy(),
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("V010"));
}

#[test]
fn broken_too_many_roles_exits_1_with_v020() {
    cmd()
        .args([
            "validate",
            &example("examples/broken/too_many_roles.yaml").to_string_lossy(),
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("V020"));
}

#[test]
fn broken_voice_fields_on_text_exits_1_with_v061() {
    cmd()
        .args([
            "validate",
            &example("examples/broken/voice_fields_on_text.yaml").to_string_lossy(),
        ])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("V061"));
}

// ===========================================================================
// CLI behavior
// ===========================================================================

#[test]
fn version_prints_version() {
    cmd()
        .args(["version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("guildforge"));
}

#[test]
fn help_lists_all_subcommands() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("apply"))
        .stdout(predicate::str::contains("destroy"))
        .stdout(predicate::str::contains("doctor"));
}

#[test]
fn nonexistent_file_exits_2() {
    cmd()
        .args(["validate", "/nonexistent/file.yaml"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn unimplemented_command_exits_2() {
    // `import` is still a stub in Phase 3.
    cmd()
        .args(["import", "12345"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("not implemented yet"));
}
