//! The suite theme tools end to end: a temporary suite folder, never the shared one.

use craft_theme_store::testing::{SAMPLE_THEME_ID, SAMPLE_THEME_JSON};
use pdfcraft_automation::{Automation, Content, SuiteDir};
use serde_json::{Value, json};

fn call(a: &mut Automation, tool: &str, args: Value) -> Result<Value, String> {
    match a.call(tool, &args).map_err(|e| e.to_string())?.into_iter().next() {
        Some(Content::Json(v)) => Ok(v),
        other => Err(format!("{tool}: unexpected result {other:?}")),
    }
}

fn ok(a: &mut Automation, tool: &str, args: Value) -> Value {
    call(a, tool, args).unwrap_or_else(|e| panic!("{tool}: {e}"))
}

#[test]
fn suite_get_and_suite_set_share_the_selection_through_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = Automation::new().with_suite_dir(SuiteDir::at(dir.path()));
    let fresh = ok(&mut a, "suite_get", json!({}));
    assert_eq!((fresh["theme"].clone(), fresh["mode"].clone(), fresh["shared"].clone()), (Value::Null, Value::Null, json!(true)));
    assert_eq!(fresh["effective"], json!({ "theme": "craft.artcraft", "mode": "dark" }), "a fresh suite shows PdfCraft's defaults");
    assert!(!dir.path().join("selection.json").exists(), "reading creates nothing");

    let set = ok(&mut a, "suite_set", json!({ "theme": "craft.graphite" }));
    assert_eq!((set["theme"].as_str(), set["mode"].clone(), set["revision"].as_u64()), (Some("craft.graphite"), Value::Null, Some(1)));
    assert_eq!(set["effective"]["mode"], "dark", "the unchosen mode stays each app's default");

    // Another process on the same folder sees it, and changes the mode on top.
    let mut b = Automation::new().with_suite_dir(SuiteDir::at(dir.path()));
    assert_eq!(ok(&mut b, "suite_get", json!({}))["theme"], "craft.graphite");
    ok(&mut b, "suite_set", json!({ "mode": "light" }));
    let both = ok(&mut a, "suite_get", json!({}));
    assert_eq!((both["theme"].as_str(), both["mode"].as_str(), both["revision"].as_u64()), (Some("craft.graphite"), Some("light"), Some(2)));

    for bad in [json!({}), json!({ "theme": "craft.nope" }), json!({ "theme": "craft.app-default" }), json!({ "mode": "sepia" }), json!({ "theme": "Bad Id" })] {
        assert!(call(&mut a, "suite_set", bad.clone()).is_err(), "{bad} is refused");
    }
    assert_eq!(ok(&mut a, "suite_get", json!({}))["revision"], 2, "refused calls write nothing");
}

#[test]
fn a_theme_imports_lists_activates_and_exports_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("sample.json"), SAMPLE_THEME_JSON).unwrap();
    let mut a = Automation::new().with_root(work.path()).unwrap().with_suite_dir(SuiteDir::at(dir.path()));

    let imported = ok(&mut a, "theme_import", json!({ "path": "sample.json" }));
    assert_eq!(imported["id"], SAMPLE_THEME_ID);
    assert_eq!(imported["warnings"]["missing"], json!([]));
    let stored = std::fs::read(dir.path().join("library").join(format!("{SAMPLE_THEME_ID}.json"))).unwrap();
    assert_eq!(stored, SAMPLE_THEME_JSON.as_bytes(), "unknown keys are kept byte for byte");

    // The same id again needs as_copy, which stores a second theme.
    let again = call(&mut a, "theme_import", json!({ "path": "sample.json" })).unwrap_err();
    assert!(again.contains("as_copy"), "{again}");
    assert_eq!(ok(&mut a, "theme_import", json!({ "path": "sample.json", "as_copy": true }))["id"], format!("{SAMPLE_THEME_ID}-2"));

    ok(&mut a, "suite_set", json!({ "theme": SAMPLE_THEME_ID }));
    let list = ok(&mut a, "theme_list", json!({}));
    let ids: Vec<&str> = list["themes"].as_array().unwrap().iter().filter_map(|t| t["id"].as_str()).collect();
    assert_eq!(&ids[..4], ["craft.app-default", "craft.artcraft", "craft.graphite", "craft.high-contrast"]);
    let sample = list["themes"].as_array().unwrap().iter().find(|t| t["id"] == SAMPLE_THEME_ID).unwrap();
    assert_eq!((sample["origin"].as_str(), sample["suite"].as_bool(), sample["variants"].clone()), (Some("library"), Some(true), json!(["light", "dark"])));

    let out = ok(&mut a, "theme_export", json!({ "id": SAMPLE_THEME_ID, "path": "out.json" }));
    assert_eq!(out["bytes"], SAMPLE_THEME_JSON.len());
    assert_eq!(std::fs::read(work.path().join("out.json")).unwrap(), SAMPLE_THEME_JSON.as_bytes());
    // A built-in exports as a theme file that imports again (as a copy: craft. ids are reserved).
    ok(&mut a, "theme_export", json!({ "id": "craft.graphite", "path": "graphite.json" }));
    let graphite: Value = serde_json::from_slice(&std::fs::read(work.path().join("graphite.json")).unwrap()).unwrap();
    assert_eq!((graphite["schema"].as_str(), graphite["light"]["text"].as_str()), (Some("craft-theme"), Some("#222226")));
    assert!(call(&mut a, "theme_export", json!({ "id": "user.missing", "path": "x.json" })).is_err());
    assert!(!work.path().join("x.json").exists());
}

#[test]
fn a_new_session_keeps_the_suite_in_memory() {
    let mut a = Automation::new();
    assert_eq!(ok(&mut a, "suite_get", json!({}))["shared"], false);
    ok(&mut a, "suite_set", json!({ "theme": "craft.high-contrast", "mode": "light" }));
    let got = ok(&mut a, "suite_get", json!({}));
    assert_eq!((got["theme"].as_str(), got["mode"].as_str()), (Some("craft.high-contrast"), Some("light")));
    assert_eq!(ok(&mut Automation::new(), "suite_get", json!({}))["theme"], Value::Null, "each session has its own");
}
