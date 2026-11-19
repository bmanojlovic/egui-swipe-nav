# egui-swipe-nav

Two-finger touchpad swipe for back and forward in [egui](https://github.com/emilk/egui)
apps, the way browsers do it.

- Swipe right goes back, swipe left goes forward. With natural scrolling the content
  follows the fingers, the same as in a browser.
- The swipe counts when the fingers lift past a threshold. Until then an arrow at the
  window edge shows how far it got, so moving the fingers back takes the swipe back.
- Fingers resting on the pad don't end the swipe; only lifting them does.
- It only collects scroll that the UI left unused. A row that scrolls sideways scrolls
  first, and navigation happens only past its end.

## Demo

```sh
cargo run --example demo
```

Open a few pages, then swipe with two fingers. The row of cards scrolls sideways
first and navigates past its ends; the list below scrolls up and down and never
navigates.

## Use

Call `update` once per frame, **after** the frame's UI, so scroll areas get the scroll
first. Pass in whether back and forward are possible right now.

```rust
// In App::ui, after the panels and pages:
match swipe.update(ui.ctx(), history.can_go_back(), history.can_go_forward()) {
    Some(egui_swipe_nav::Direction::Back) => history.back(),
    Some(egui_swipe_nav::Direction::Forward) => history.forward(),
    None => {}
}
swipe.paint(ui.ctx(), content_rect);
```

`SwipeNav::default()` has a 120 point threshold. `threshold`, `min_horizontal_ratio`,
`idle_end` and `lost_end` are public fields.

## How it reads the gesture

The touchpad gesture arrives as `egui::Event::MouseWheel` with point deltas and a
`phase` (egui 0.37+).

- **Phases reported** (winit on Wayland and macOS): the swipe ends on `TouchPhase::End`.
  If the end never comes, the swipe is dropped after `lost_end` seconds without
  navigating.
- **No phases:** the swipe ends after `idle_end` seconds without scroll events.
- **Line deltas** (mouse wheels) are ignored.

## License

MIT OR Apache-2.0, at your option.
