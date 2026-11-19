# Changelog

## 0.1.0 - 2026-11-19

Initial release.

- `SwipeNav`: a two-finger touchpad swipe goes back (right) or forward (left), counted when
  the fingers lift past a threshold. It reads egui 0.37's wheel phases and falls back to an
  idle gap where there are none.
- Uses only horizontal scroll the frame's UI left unused, so sideways scroll areas scroll
  first.
- `SwipeNav::paint`: an arrow at the window edge that grows with the swipe.
- `SwipeNav::gesture`: a snapshot of the swipe under way, for showing or debugging it.
- `examples/demo.rs`: pages to swipe between, with a meter.
