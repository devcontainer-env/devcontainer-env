use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::{path::PathBuf, sync::LazyLock};

static WORKSPACE_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
});

#[test]
fn test_inspect_devcontainer_default() {
    cargo_bin_cmd!()
        .current_dir(WORKSPACE_FOLDER.to_path_buf())
        .args(["inspect"])
        .assert()
        .success()
        .stdout(predicate::str::contains("devcontainer-env"));
}
