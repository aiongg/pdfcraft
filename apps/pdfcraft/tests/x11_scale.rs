#[test]
fn the_desktop_file_leaves_the_x11_scale_to_winit() {
    // winit (craft/0.30.13-lang) takes the scale from Xft.dpi, then the X screen's configured DPI. A
    // WINIT_X11_SCALE_FACTOR in the launcher would override both.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let desktop = std::fs::read_to_string(root.join("packaging/linux/ai.storyteller.pdfcraft.desktop")).unwrap();
    assert!(!desktop.contains("WINIT_X11_SCALE_FACTOR"));
}
