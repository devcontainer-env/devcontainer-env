use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::path::Path;

fn devcontainer_env() -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap(),
    );
    cmd
}

#[test]
fn inspect_default() {
    devcontainer_env()
        .args(["inspect"])
        .assert()
        .success()
        .stdout(predicate::str::contains("devcontainer-env"));
}

#[test]
fn inspect_with_workspace_folder() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    cargo_bin_cmd!()
        .args([
            "inspect",
            "--workspace-folder",
            workspace.to_str().unwrap(),
            "--config",
            workspace
                .join(".devcontainer/devcontainer.json")
                .to_str()
                .unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("devcontainer-env"));
}

#[test]
fn inspect_fails_when_config_missing() {
    let tmp = std::env::temp_dir();
    cargo_bin_cmd!()
        .current_dir(tmp)
        .args(["inspect"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));
}

#[test]
fn export_writes_bash_statements() {
    devcontainer_env()
        .args(["export", "--format", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export DATABASE_URL=postgres://"));
}

#[test]
fn export_writes_json_object() {
    devcontainer_env()
        .args(["export", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"DATABASE_URL\":"));
}

#[test]
fn exec_runs_command_with_env() {
    devcontainer_env()
        .args(["exec", "env"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DATABASE_URL=postgres://"));
}

#[test]
fn exec_fails_when_command_exits_nonzero() {
    devcontainer_env()
        .args(["exec", "sh", "-c", "exit 1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("command exited with status"));
}

#[test]
fn exec_fails_when_command_not_found() {
    devcontainer_env()
        .args(["exec", "non-existent-command-12345"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));
}
