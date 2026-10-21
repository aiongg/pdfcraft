#[test]
fn the_empty_tab_strip_moves_the_window() {
    // On macOS the tab strip is the title bar, and winit (craft/0.30.13-lang) leaves drags that begin
    // there to the app so document tabs can be reordered. The strip's empty background must then
    // move the window itself.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let chrome = std::fs::read_to_string(root.join("crates/ui-egui/src/chrome.rs")).unwrap();
    assert!(chrome.contains("ViewportCommand::StartDrag"), "the empty tab strip no longer moves the window");
}
