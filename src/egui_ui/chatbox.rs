use std::sync::Arc;
use std::sync::Mutex;

use egui::{Response, Sense, Ui, Widget};
use egui_extras::{Size, TableBuilder};
use glutin::event_loop::EventLoopProxy;
use twitch_irc::message::ServerMessage;
use twitch_irc::message::PrivmsgMessage;

use crate::egui_ui::BotfaceEvent;
use crate::irc::MessageDispatcher;

struct ChatMessage {
    user: String,
    message: String,
}

impl From<PrivmsgMessage> for ChatMessage {
    fn from(msg: PrivmsgMessage) -> Self {
        Self {
            user: msg.sender.name,
            message: msg.message_text,
        }
    }
}

pub struct ChatboxState {
    messages: Vec<ChatMessage>,
}

impl ChatboxState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

pub struct Chatbox {
    state: Arc<Mutex<ChatboxState>>,
}

impl Chatbox {
    pub fn new(state: Arc<Mutex<ChatboxState>>) -> Self {
        Self { state }
    }

    pub fn state(&self) -> Arc<Mutex<ChatboxState>> {
        self.state.clone()
    }
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

pub struct ChatboxDispatcher {
    message_dispatcher: MessageDispatcher,
    state: Arc<Mutex<ChatboxState>>,
    proxy: EventLoopProxy<BotfaceEvent>,
}

impl ChatboxDispatcher {
    pub fn new(
        message_dispatcher: MessageDispatcher,
        state: Arc<Mutex<ChatboxState>>,
        proxy: EventLoopProxy<BotfaceEvent>,
    ) -> Self {
        Self {
            message_dispatcher,
            state,
            proxy,
        }
    }

    pub async fn run(&mut self) {
        while let Ok(message) = self.message_dispatcher.receiver.recv().await {
            match message {
                ServerMessage::Privmsg(msg) => {
                    match self.state.lock() {
                        Ok(mut cbstate) => (*cbstate).messages.push(msg.into()),
                        Err(e) => eprintln!("{:?}", e),
                    }
                    self.proxy.send_event(BotfaceEvent::Nonce);
                }
                _ => (),
            }
        }
    }
}
