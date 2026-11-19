//! Swipe back and forward between pages with two fingers on a touchpad.
//!
//! `cargo run --example demo`
//!
//! Open a few pages with the buttons, then swipe right to go back and left to
//! go forward. The row of cards scrolls sideways first; past its end the same
//! swipe navigates. Scrolling the list up and down never navigates.

use eframe::egui;
use egui_swipe_nav::{Direction, SwipeNav};

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
            egui::ScrollArea::vertical().id_salt("rows").show(ui, |ui| {
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
        self.swipe.paint(ui.ctx(), content);
    }
}
