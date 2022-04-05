use egui::{Response, Sense, Ui, Widget};
use egui_extras::{Size, TableBuilder};
use glutin::event_loop::EventLoopProxy;

use crate::irc::MessageDispatcher;
use crate::egui_ui::BotfaceEvent;

struct ChatMessage {
    user: String,
    message: String,
}

pub struct Chatbox {
    message_dispatcher: MessageDispatcher,
    messages: Vec<String>,
}

impl Chatbox {
    pub fn new(message_dispatcher: MessageDispatcher) -> Self {
        Self {
            message_dispatcher,
            messages: Vec::new(),
        }
    }

    pub async fn run(&self, proxy: EventLoopProxy<BotfaceEvent>) {}
}

impl<'a> Widget for &'a mut Chatbox {
    fn ui(self, ui: &mut Ui) -> Response {
        let inner_response = egui::TopBottomPanel::bottom("chat area")
            .resizable(true)
            .show_inside(ui, |bottom_ui| {
                bottom_ui.label("meow");
            });
        egui::CentralPanel::default().show_inside(ui, |top_ui| {
            TableBuilder::new(top_ui)
                .striped(true)
                .column(Size::relative(1.0))
                .body(|mut body| {
                    let row_heights: Vec<f32> =
                        vec![20.0, 20.0, 100.0, 20.0, 20.0, 100.0, 20.0, 20.0, 100.0];
                    body.heterogeneous_rows(row_heights.into_iter(), |_index, mut row| {
                        row.col(|ui| {
                            ui.label("hello cruel world");
                        });
                    });
                });
        });
        inner_response.response
    }
}
