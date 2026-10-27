//! `pdfcraft-cli run` keeps the suite theme in memory unless a suite folder is named, and always
//! under `--root`. HOME and XDG_CONFIG_HOME point into a temporary folder, so even a wrong
//! answer never reaches the real suite folder.

use std::path::Path;
use std::process::Command;

fn suite_set(home: &Path, env_dir: Option<&Path>, args: &[&str]) -> (bool, serde_json::Value, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_pdfcraft-cli"));
    cmd.env("HOME", home).env("XDG_CONFIG_HOME", home.join("config")).env_remove("CRAFT_SUITE_DIR");
    if let Some(dir) = env_dir {
        cmd.env("CRAFT_SUITE_DIR", dir);
    }
    cmd.args(["run", "suite_set", "theme=craft.graphite"]).args(args);
    let out = cmd.output().unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (out.status.success(), json, String::from_utf8_lossy(&out.stderr).into_owned())
}

fn written(dir: &Path) -> bool {
    dir.join("selection.json").exists()
}

#[test]
fn the_suite_folder_is_used_only_when_named_and_never_under_root() {
    let base = std::env::temp_dir().join(format!("pdfcraft-cli-suite-dir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let home = &base;
    let (suite, root, other) = (base.join("suite"), base.join("root"), base.join("other"));
    for dir in [&suite, &root, &other] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let platform = home.as_path().join("config/ArtCraft/themes");

    // Nothing named: in memory, not the platform folder.
    let (ok, json, err) = suite_set(home.as_path(), None, &[]);
    assert!(ok, "{err}");
    assert_eq!(json["shared"], false);
    assert!(!written(&platform));

    // Under --root, CRAFT_SUITE_DIR is ignored and --suite-dir refused.
    let (ok, json, err) = suite_set(home.as_path(), Some(&suite), &["--root", root.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert_eq!(json["shared"], false);
    assert!(!written(&suite));
    let (ok, _, err) = suite_set(home.as_path(), None, &["--root", root.to_str().unwrap(), "--suite-dir", suite.to_str().unwrap()]);
    assert!(!ok && err.contains("--suite-dir"), "{err}");
    assert!(!written(&suite));

    // Named by the environment or the option: the folder.
    let (ok, json, err) = suite_set(home.as_path(), Some(&suite), &[]);
    assert!(ok, "{err}");
    assert_eq!(json["shared"], true);
    assert!(written(&suite));
    let (ok, json, err) = suite_set(home.as_path(), None, &["--suite-dir", other.to_str().unwrap()]);
    assert!(ok, "{err}");
    assert_eq!(json["shared"], true);
    assert!(written(&other));
    assert!(!written(&platform));
    let _ = std::fs::remove_dir_all(&base);
}
