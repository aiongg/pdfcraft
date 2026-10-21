//! How egui treats the wheel phases winit (craft/suite) reports on Wayland: pixel-only wheel,
//! wheel-tilt and continuous-source events arrive as standalone `Moved` events, finger scrolls as a
//! gesture from `Started` (#991). winit tests its phase policy itself.

use egui::{Event, Modifiers, MouseWheelUnit, RawInput, TouchPhase, vec2};

fn wheel_trace(phases: [TouchPhase; 3], discrete: bool) -> Vec<(f32, egui::Vec2)> {
    let ctx = egui::Context::default();
    let mut seen = Vec::new();
    let ctrl = Modifiers { ctrl: true, command: true, ..Modifiers::NONE };
    for (frame, (phase, modifiers)) in phases.into_iter().zip([ctrl, ctrl, Modifiers::NONE]).enumerate() {
        let (unit, dy) = if discrete { (MouseWheelUnit::Line, 1.0) } else { (MouseWheelUnit::Point, 24.0) };
        let input = RawInput {
            time: Some(frame as f64 / 60.0),
            events: vec![Event::MouseWheel { unit, delta: vec2(0.0, dy), phase, modifiers, source: egui::MouseWheelSource::Unknown }],
            ..Default::default()
        };
        ctx.run_ui(input, |ui| seen.push(ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta)))).textures_delta.clear();
    }
    seen
}

#[test]
fn releasing_ctrl_after_pixel_wheel_zoom_returns_to_scrolling() {
    let seen = wheel_trace([TouchPhase::Move; 3], false);
    assert!(seen[1].0 > 1.0, "Ctrl must zoom");
    assert_eq!(seen[2].0, 1.0, "releasing Ctrl must stop zooming");
    assert!(seen[2].1.y > 0.0, "the next wheel event must scroll");
}

#[test]
fn discrete_wheels_still_follow_current_modifiers() {
    let seen = wheel_trace([TouchPhase::Move; 3], true);
    assert!(seen[1].0 > 1.0);
    assert_eq!(seen[2].0, 1.0);
    assert!(seen[2].1.y > 0.0);
}

#[test]
fn finger_gestures_keep_their_modifier_until_the_gesture_ends() {
    // A finger scroll's momentum may arrive after Ctrl is released.
    let seen = wheel_trace([TouchPhase::Start, TouchPhase::Move, TouchPhase::Move], false);
    assert!(seen[1].0 > 1.0);
    assert!(seen[2].0 > 1.0);
    assert_eq!(seen[2].1, egui::Vec2::ZERO);
}
