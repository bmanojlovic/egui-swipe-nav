//! Swipe back and forward between pages with two fingers on a touchpad.
//!
//! `cargo run --example demo`
//!
//! Open a few pages with the buttons, then swipe right to go back and left to
//! go forward. The row of cards scrolls sideways first; past its end the same
//! swipe navigates. Scrolling the list up and down never navigates.

use eframe::egui;
use egui_swipe_nav::{Direction, GestureInfo, SwipeNav};

fn main() -> eframe::Result {
    eframe::run_native(
        "egui-swipe-nav demo",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::<Demo>::default())),
    )
}

#[derive(Default)]
struct Demo {
    page: usize,
    back: Vec<usize>,
    forward: Vec<usize>,
    swipe: SwipeNav,
    last: Option<Direction>,
    /// The last swipe seen, kept on the meter after it ends.
    held: Option<GestureInfo>,
}

impl Demo {
    fn open(&mut self, page: usize) {
        self.back.push(self.page);
        self.forward.clear();
        self.page = page;
    }

    fn step(&mut self, direction: Direction) {
        let (from, to) = match direction {
            Direction::Back => (&mut self.back, &mut self.forward),
            Direction::Forward => (&mut self.forward, &mut self.back),
        };
        if let Some(page) = from.pop() {
            to.push(self.page);
            self.page = page;
            self.last = Some(direction);
        }
    }
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut content = egui::Rect::NOTHING;
        egui::CentralPanel::default().show(ui, |ui| {
            content = ui.max_rect();
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.back.is_empty(), egui::Button::new("⏴ Back")).clicked() {
                    self.step(Direction::Back);
                }
                if ui
                    .add_enabled(!self.forward.is_empty(), egui::Button::new("Forward ⏵"))
                    .clicked()
                {
                    self.step(Direction::Forward);
                }
                ui.heading(format!("Page {}", self.page));
            });
            ui.label(match self.last {
                Some(direction) => format!("Last step: {direction:?}"),
                None => "Open a page, then swipe right with two fingers to go back.".into(),
            });
            gesture_meter(ui, &self.swipe, self.held);
            ui.separator();

            ui.horizontal_wrapped(|ui| {
                let current = self.page;
                for page in (1..=6).filter(|&page| page != current) {
                    if ui.button(format!("Open page {page}")).clicked() {
                        self.open(page);
                    }
                }
            });
            ui.add_space(12.0);

            ui.label("Scrolls sideways first, navigates past its ends:");
            egui::ScrollArea::horizontal().id_salt("cards").show(ui, |ui| {
                ui.horizontal(|ui| {
                    for card in 1..=20 {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(120.0, 80.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 6.0, ui.visuals().faint_bg_color);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("Card {card}"),
                            egui::FontId::proportional(14.0),
                            ui.visuals().text_color(),
                        );
                    }
                });
            });
            ui.add_space(12.0);

            ui.label("Scrolls up and down, never navigates:");
            egui::ScrollArea::vertical().id_salt("rows").auto_shrink(false).show(ui, |ui| {
                for row in 1..=60 {
                    ui.label(format!("Page {} row {row}", self.page));
                }
            });
        });

        // After the UI, so the scroll areas above get the scroll first.
        let (can_go_back, can_go_forward) = (!self.back.is_empty(), !self.forward.is_empty());
        if let Some(direction) = self.swipe.update(ui.ctx(), can_go_back, can_go_forward) {
            self.step(direction);
        }
        if let Some(gesture) = self.swipe.gesture() {
            self.held = Some(gesture);
        }
        self.swipe.paint(ui.ctx(), content);
    }
}

/// A bar from forward (left) to back (right) showing how far the swipe got
/// against the threshold, with what the recognizer sees in words beside it.
fn gesture_meter(ui: &mut egui::Ui, swipe: &SwipeNav, held: Option<GestureInfo>) {
    let live = swipe.gesture();
    let shown = live.or(held);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(320.0, 22.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let visuals = ui.visuals();
        painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);
        let center = rect.center().x;
        // The threshold sits at three quarters of each half.
        let scale = rect.width() / 2.0 * 0.75 / swipe.threshold;
        for side in [-1.0, 1.0] {
            let x = center + side * swipe.threshold * scale;
            painter.vline(x, rect.y_range(), egui::Stroke::new(1.0, visuals.weak_text_color()));
        }
        painter.vline(center, rect.y_range(), egui::Stroke::new(1.0, visuals.text_color()));
        if let Some(gesture) = shown {
            let reached = gesture.leftover.abs() >= swipe.threshold;
            let color = match (live.is_some(), reached && gesture.direction.is_some()) {
                (false, _) => visuals.weak_text_color(),
                (true, true) => egui::Color32::from_rgb(80, 200, 120),
                (true, false) => visuals.selection.bg_fill,
            };
            let end = (center + gesture.leftover * scale).clamp(rect.left(), rect.right());
            let bar = egui::Rect::from_x_y_ranges(
                center.min(end)..=center.max(end),
                rect.shrink(4.0).y_range(),
            );
            painter.rect_filled(bar, 2.0, color);
        }
        painter.text(
            rect.left_center() + egui::vec2(6.0, 0.0),
            egui::Align2::LEFT_CENTER,
            "forward",
            egui::FontId::proportional(11.0),
            visuals.weak_text_color(),
        );
        painter.text(
            rect.right_center() - egui::vec2(6.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "back",
            egui::FontId::proportional(11.0),
            visuals.weak_text_color(),
        );

        ui.label(match shown {
            None => "no swipe yet".to_string(),
            Some(gesture) => format!(
                "{} {:.0}/{:.0} pt  moved x {:.0} y {:.0}  {}  {}",
                match gesture.direction {
                    Some(Direction::Back) => "back",
                    Some(Direction::Forward) => "forward",
                    None => "-",
                },
                gesture.leftover.abs(),
                swipe.threshold,
                gesture.moved.x,
                gesture.moved.y,
                if live.is_none() {
                    "ended"
                } else if gesture.fingers_down {
                    "fingers down"
                } else {
                    "fingers lifted"
                },
                if gesture.phased { "phases" } else { "no phases" },
            ),
        });
    });
}
