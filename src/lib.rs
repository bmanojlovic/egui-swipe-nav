//! Two-finger touchpad swipe for back and forward, the way browsers do it.
//!
//! Swiping right goes back, left goes forward (with natural scrolling: the
//! content follows the fingers, as in a browser). A swipe counts once the
//! fingers lift past [`SwipeNav::threshold`]; until then an arrow at the
//! window edge shows how far it got, so a swipe can be taken back by moving
//! the fingers back.
//!
//! It only sees what the UI left over. Call [`SwipeNav::update`] after the
//! UI for the frame has run: a scroll area that scrolls sideways takes the
//! horizontal delta while it can still move, and the swipe only collects
//! what is left, which is the overscroll at its ends. So a sideways row
//! scrolls first and navigates only past its end, as in a browser.
//!
//! The touchpad gesture comes in as `Event::MouseWheel` with point deltas.
//! Where the platform reports the gesture phase (winit does on Wayland and
//! macOS), the swipe ends only when the fingers lift: fingers resting on
//! the pad send nothing, and that must not count as letting go. Elsewhere
//! it ends after [`SwipeNav::idle_end`] without scroll events. Line-based
//! deltas (mouse wheels) are ignored.
//!
//! ```no_run
//! # fn app(ui: &mut egui::Ui, swipe: &mut egui_swipe_nav::SwipeNav) {
//! // ... the app's panels and pages first ...
//! match swipe.update(ui.ctx(), true, true) {
//!     Some(egui_swipe_nav::Direction::Back) => { /* go back */ }
//!     Some(egui_swipe_nav::Direction::Forward) => { /* go forward */ }
//!     None => {}
//! }
//! swipe.paint(ui.ctx(), ui.ctx().content_rect());
//! # }
//! ```

use egui::{Color32, Context, Event, MouseWheelUnit, Pos2, Rect, Stroke, TouchPhase, Vec2};

/// Which way a swipe goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Back,
    Forward,
}

/// Recognizes the swipe across frames. Keep one per window.
#[derive(Clone, Debug)]
pub struct SwipeNav {
    /// How far (in points) the leftover horizontal scroll has to go for the
    /// swipe to count.
    pub threshold: f32,
    /// How much more horizontal than vertical the fingers have to move, so
    /// a slightly slanted vertical scroll never navigates.
    pub min_horizontal_ratio: f32,
    /// Without phase events: the swipe ends after this long without any
    /// scroll event.
    pub idle_end: f64,
    /// With phase events: a swipe whose `End` hasn't come after this long
    /// without events is dropped, without navigating.
    pub lost_end: f64,
    gesture: Option<Gesture>,
    allowed: (bool, bool),
}

#[derive(Clone, Copy, Debug)]
struct Gesture {
    /// Horizontal scroll nothing else used.
    leftover: f32,
    /// Everything the fingers did, used or not, to tell the direction.
    moved: Vec2,
    last_event: f64,
    fingers_lifted: bool,
    /// The platform reported the start, so it will report the end too.
    phased: bool,
}

impl Default for SwipeNav {
    fn default() -> Self {
        Self {
            threshold: 120.0,
            min_horizontal_ratio: 1.5,
            idle_end: 0.3,
            lost_end: 5.0,
            gesture: None,
            allowed: (false, false),
        }
    }
}

impl SwipeNav {
    /// Feeds one frame. Call after the frame's UI, with whether going back
    /// and forward are possible right now; a swipe towards a side that is
    /// not possible is neither shown nor reported. Returns the direction
    /// once, on the frame the swipe completes.
    pub fn update(&mut self, ctx: &Context, can_go_back: bool, can_go_forward: bool) -> Option<Direction> {
        self.allowed = (can_go_back, can_go_forward);
        let (time, leftover, wheel) = ctx.input(|i| {
            let wheel: Vec<(Vec2, TouchPhase)> = i
                .events
                .iter()
                .filter_map(|event| match event {
                    Event::MouseWheel { unit: MouseWheelUnit::Point, delta, phase, .. } => {
                        Some((*delta, *phase))
                    }
                    _ => None,
                })
                .collect();
            (i.time, i.smooth_scroll_delta.x, wheel)
        });
        let finished = self.feed(time, leftover, &wheel);
        if self.gesture.is_some() {
            // The idle end needs a frame after the last event.
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(self.idle_end / 2.0));
        }
        finished
    }

    /// The swipe so far: its direction and how far it got, 1.0 being enough
    /// to count. `None` while there is no swipe or it goes nowhere allowed.
    pub fn progress(&self) -> Option<(Direction, f32)> {
        let gesture = self.gesture?;
        let direction = self.direction(gesture)?;
        Some((direction, (gesture.leftover.abs() / self.threshold).min(1.0)))
    }

    /// Draws the arrow at the left (back) or right (forward) edge of `area`
    /// while a swipe is under way: visible from the first movement, growing
    /// and brightening with the swipe, solid once letting go would count.
    pub fn paint(&self, ctx: &Context, area: Rect) {
        let Some((direction, progress)) = self.progress() else {
            return;
        };
        if progress <= 0.0 {
            return;
        }
        let arrow = Arrow::new(direction, progress, area);
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("egui-swipe-nav"),
        ));
        let done = progress >= 1.0;
        let fill = if done { Color32::from_gray(235) } else { Color32::from_gray(70) };
        let ink = if done { Color32::from_gray(20) } else { Color32::from_gray(235) };
        painter.circle_filled(arrow.center, arrow.radius, fill.gamma_multiply(arrow.alpha));
        let tip = match direction {
            Direction::Back => -1.0,
            Direction::Forward => 1.0,
        };
        let size = arrow.radius / 22.0;
        let stroke = Stroke::new(2.5 * size, ink.gamma_multiply(arrow.alpha));
        let center = arrow.center;
        let head = center + Vec2::new(tip * 8.0 * size, 0.0);
        painter.line_segment([center - Vec2::new(tip * 9.0 * size, 0.0), head], stroke);
        painter.line_segment([head, head + Vec2::new(-tip * 7.0, -7.0) * size], stroke);
        painter.line_segment([head, head + Vec2::new(-tip * 7.0, 7.0) * size], stroke);
    }

    /// The recognizer itself, separate from egui's input so tests can drive
    /// it: `leftover` is this frame's unused horizontal scroll, `wheel` the
    /// frame's point-unit wheel events.
    fn feed(&mut self, time: f64, leftover: f32, wheel: &[(Vec2, TouchPhase)]) -> Option<Direction> {
        for &(delta, phase) in wheel {
            match phase {
                TouchPhase::Start => {
                    self.gesture = Some(Gesture { phased: true, ..Gesture::new(time) });
                }
                TouchPhase::Move => {
                    let gesture = self.gesture.get_or_insert_with(|| Gesture::new(time));
                    if gesture.fingers_lifted {
                        // Scrolling again before the last one settled.
                        *gesture = Gesture::new(time);
                    }
                }
                TouchPhase::End | TouchPhase::Cancel => {}
            }
            if let Some(gesture) = &mut self.gesture {
                gesture.moved += delta;
                gesture.last_event = time;
                match phase {
                    TouchPhase::End => gesture.fingers_lifted = true,
                    // Cancelled by the system: never navigates.
                    TouchPhase::Cancel => self.gesture = None,
                    TouchPhase::Start | TouchPhase::Move => {}
                }
            }
        }

        let gesture = self.gesture.as_mut()?;
        gesture.leftover += leftover;
        // egui hands out a scroll over a few frames; with the fingers up,
        // wait for the rest of it before deciding.
        let settled = gesture.fingers_lifted && leftover == 0.0;
        let quiet = time - gesture.last_event;
        if gesture.phased && !gesture.fingers_lifted {
            if quiet > self.lost_end {
                self.gesture = None;
            }
            return None;
        }
        if !(settled || quiet > self.idle_end) {
            return None;
        }
        let gesture = self.gesture.take()?;
        let direction = self.direction(gesture)?;
        (gesture.leftover.abs() >= self.threshold).then_some(direction)
    }

    fn direction(&self, gesture: Gesture) -> Option<Direction> {
        let horizontal = gesture.moved.x.abs() >= self.min_horizontal_ratio * gesture.moved.y.abs();
        if !horizontal || gesture.leftover == 0.0 {
            return None;
        }
        // Content moving right means the fingers moved right: back.
        let (direction, allowed) = if gesture.leftover > 0.0 {
            (Direction::Back, self.allowed.0)
        } else {
            (Direction::Forward, self.allowed.1)
        };
        allowed.then_some(direction)
    }
}

/// Where and how the arrow is drawn for a swipe that got `progress` of the
/// way.
struct Arrow {
    center: Pos2,
    radius: f32,
    alpha: f32,
}

impl Arrow {
    const RADIUS: f32 = 22.0;
    const MARGIN: f32 = 12.0;
    const TRAVEL: f32 = 24.0;

    fn new(direction: Direction, progress: f32, area: Rect) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        let radius = Self::RADIUS * (0.6 + 0.4 * progress);
        let inset = Self::MARGIN + radius + Self::TRAVEL * progress;
        let x = match direction {
            Direction::Back => area.left() + inset,
            Direction::Forward => area.right() - inset,
        };
        Self { center: Pos2::new(x, area.center().y), radius, alpha: 0.4 + 0.6 * progress }
    }
}

impl Gesture {
    fn new(time: f64) -> Self {
        Self { leftover: 0.0, moved: Vec2::ZERO, last_event: time, fingers_lifted: false, phased: false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f64 = 1.0 / 60.0;

    /// Plays a swipe frame by frame: every frame one wheel event of `step`
    /// that nothing else uses, then the fingers lift.
    fn swipe(nav: &mut SwipeNav, step: Vec2, frames: usize) -> Option<Direction> {
        let mut time = 0.0;
        let mut result = None;
        for frame in 0..frames {
            let phase = if frame == 0 { TouchPhase::Start } else { TouchPhase::Move };
            result = result.or(nav.feed(time, step.x, &[(step, phase)]));
            time += FRAME;
        }
        result = result.or(nav.feed(time, 0.0, &[(Vec2::ZERO, TouchPhase::End)]));
        result.or(nav.feed(time + FRAME, 0.0, &[]))
    }

    fn nav() -> SwipeNav {
        SwipeNav { allowed: (true, true), ..SwipeNav::default() }
    }

    #[test]
    fn swiping_right_far_enough_goes_back() {
        assert_eq!(swipe(&mut nav(), Vec2::new(15.0, 0.0), 10), Some(Direction::Back));
    }

    #[test]
    fn swiping_left_far_enough_goes_forward() {
        assert_eq!(swipe(&mut nav(), Vec2::new(-15.0, 0.0), 10), Some(Direction::Forward));
    }

    #[test]
    fn a_short_swipe_does_nothing() {
        assert_eq!(swipe(&mut nav(), Vec2::new(10.0, 0.0), 5), None);
    }

    #[test]
    fn a_mostly_vertical_scroll_does_nothing() {
        assert_eq!(swipe(&mut nav(), Vec2::new(15.0, 20.0), 20), None);
    }

    #[test]
    fn nothing_happens_before_the_fingers_lift() {
        let mut nav = nav();
        let step = Vec2::new(30.0, 0.0);
        let mut time = 0.0;
        for frame in 0..10 {
            let phase = if frame == 0 { TouchPhase::Start } else { TouchPhase::Move };
            assert_eq!(nav.feed(time, step.x, &[(step, phase)]), None);
            time += FRAME;
        }
        assert_eq!(nav.progress(), Some((Direction::Back, 1.0)));
    }

    #[test]
    fn moving_the_fingers_back_takes_the_swipe_back() {
        let mut nav = nav();
        let mut time = 0.0;
        let out = Vec2::new(30.0, 0.0);
        nav.feed(time, out.x, &[(out, TouchPhase::Start)]);
        for _ in 0..5 {
            time += FRAME;
            nav.feed(time, out.x, &[(out, TouchPhase::Move)]);
        }
        for _ in 0..6 {
            time += FRAME;
            nav.feed(time, -out.x, &[(-out, TouchPhase::Move)]);
        }
        time += FRAME;
        assert_eq!(nav.feed(time, 0.0, &[(Vec2::ZERO, TouchPhase::End)]), None);
    }

    #[test]
    fn scroll_used_by_the_ui_does_not_count() {
        // A sideways row took all of it: nothing left over.
        let mut nav = nav();
        let step = Vec2::new(30.0, 0.0);
        let mut time = 0.0;
        for frame in 0..10 {
            let phase = if frame == 0 { TouchPhase::Start } else { TouchPhase::Move };
            assert_eq!(nav.feed(time, 0.0, &[(step, phase)]), None);
            time += FRAME;
        }
        assert_eq!(nav.feed(time, 0.0, &[(Vec2::ZERO, TouchPhase::End)]), None);
    }

    #[test]
    fn a_side_with_nowhere_to_go_is_ignored() {
        let mut nav = SwipeNav { allowed: (false, true), ..SwipeNav::default() };
        assert_eq!(swipe(&mut nav, Vec2::new(15.0, 0.0), 10), None);
    }

    #[test]
    fn a_cancelled_gesture_does_nothing() {
        let mut nav = nav();
        let step = Vec2::new(30.0, 0.0);
        nav.feed(0.0, step.x, &[(step, TouchPhase::Start)]);
        nav.feed(FRAME, step.x, &[(step, TouchPhase::Move)]);
        nav.feed(2.0 * FRAME, step.x, &[(step, TouchPhase::Move)]);
        nav.feed(3.0 * FRAME, step.x, &[(step, TouchPhase::Move)]);
        nav.feed(4.0 * FRAME, step.x, &[(step, TouchPhase::Move)]);
        assert_eq!(nav.feed(5.0 * FRAME, 0.0, &[(Vec2::ZERO, TouchPhase::Cancel)]), None);
        assert_eq!(nav.feed(6.0 * FRAME, 0.0, &[]), None);
    }

    /// Fingers that stop on the pad send nothing. That is not letting go.
    #[test]
    fn resting_fingers_do_not_end_the_swipe() {
        let mut nav = nav();
        let step = Vec2::new(30.0, 0.0);
        let mut time = 0.0;
        for frame in 0..6 {
            let phase = if frame == 0 { TouchPhase::Start } else { TouchPhase::Move };
            nav.feed(time, step.x, &[(step, phase)]);
            time += FRAME;
        }
        for _ in 0..120 {
            time += FRAME;
            assert_eq!(nav.feed(time, 0.0, &[]), None, "fired while the fingers rest");
        }
        // Letting go is what counts.
        assert_eq!(nav.feed(time, 0.0, &[(Vec2::ZERO, TouchPhase::End)]), Some(Direction::Back));
    }

    #[test]
    fn a_swipe_whose_end_never_comes_is_dropped_without_navigating() {
        let mut nav = nav();
        let step = Vec2::new(30.0, 0.0);
        nav.feed(0.0, step.x, &[(step, TouchPhase::Start)]);
        for frame in 1..6 {
            nav.feed(frame as f64 * FRAME, step.x, &[(step, TouchPhase::Move)]);
        }
        assert_eq!(nav.feed(10.0, 0.0, &[]), None);
        assert_eq!(nav.progress(), None, "the swipe is still held");
    }

    #[test]
    fn the_arrow_is_on_screen_from_the_first_movement() {
        let area = Rect::from_min_size(Pos2::new(100.0, 0.0), Vec2::new(800.0, 600.0));
        for direction in [Direction::Back, Direction::Forward] {
            let arrow = Arrow::new(direction, 0.05, area);
            assert!(arrow.center.x - arrow.radius >= area.left(), "{direction:?} starts off screen");
            assert!(arrow.center.x + arrow.radius <= area.right(), "{direction:?} starts off screen");
            assert!(arrow.alpha >= 0.4);
            let done = Arrow::new(direction, 1.0, area);
            assert!(done.radius > arrow.radius && done.alpha > arrow.alpha);
        }
    }

    #[test]
    fn without_phases_the_swipe_ends_when_the_events_stop() {
        let mut nav = nav();
        let step = Vec2::new(15.0, 0.0);
        let mut time = 0.0;
        for _ in 0..10 {
            assert_eq!(nav.feed(time, step.x, &[(step, TouchPhase::Move)]), None);
            time += FRAME;
        }
        assert_eq!(nav.feed(time + 0.1, 0.0, &[]), None, "ended too early");
        assert_eq!(nav.feed(time + 0.5, 0.0, &[]), Some(Direction::Back));
    }

    #[test]
    fn the_end_waits_for_the_scroll_egui_still_hands_out() {
        let mut nav = nav();
        let step = Vec2::new(15.0, 0.0);
        nav.feed(0.0, 0.0, &[(step * 9.0, TouchPhase::Start)]);
        // The fingers lift while most of the delta is still to come.
        assert_eq!(nav.feed(FRAME, 60.0, &[(Vec2::ZERO, TouchPhase::End)]), None);
        assert_eq!(nav.feed(2.0 * FRAME, 75.0, &[]), None);
        assert_eq!(nav.feed(3.0 * FRAME, 0.0, &[]), Some(Direction::Back));
    }
}
