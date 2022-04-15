use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;

use egui::text::LayoutJob;
use egui::{Color32, Context, FontFamily, FontId, Response, Ui};
use egui_extras::{Size, TableBuilder};
use epaint::text::TextWrapping;
use epaint::text::{Fonts, Galley, TextFormat};
use glutin::event_loop::EventLoopProxy;
use lock_api::MappedRwLockReadGuard;
use twitch_irc::message::PrivmsgMessage;
use twitch_irc::message::ServerMessage;

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

impl From<&ChatMessage> for String {
    fn from(msg: &ChatMessage) -> String {
        format!("{}: {}", msg.user, msg.message)
    }
}

impl fmt::Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
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
    rendered_messages: Vec<Arc<Galley>>,
    width: f32,
}

// public fns
impl Chatbox {
    pub fn new(state: Arc<Mutex<ChatboxState>>) -> Self {
        Self {
            state,
            rendered_messages: Vec::new(),
            width: 0.0,
        }
    }

    pub fn state(&self) -> Arc<Mutex<ChatboxState>> {
        self.state.clone()
    }

    pub fn show(&mut self, ui: &mut Ui, egui_ctx: Context) -> Response {
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
                    let widths = body.widths();
                    if self.width != widths[0] {
                        self.width = widths[0];
                        self.rendered_messages = Vec::new();
                    }
                    self.convert_new_messages(&egui_ctx);
                    let row_height_iter = self.rendered_messages.iter().map(|galley| {
                        let height = galley.size().y;
                        height
                    });
                    body.heterogeneous_rows(row_height_iter, |i, mut row| {
                        row.col(|ui| {
                            ui.label(self.rendered_messages[i].clone());
                        });
                    });
                });
        });
        inner_response.response
    }
}

// non-public fns
impl Chatbox {
    fn message_to_layout_job(&self, msg: &ChatMessage) -> LayoutJob {
        let mut job = LayoutJob::single_section(
            msg.into(),
            TextFormat {
                font_id: FontId::new(18.0, FontFamily::Monospace),
                color: Color32::WHITE,
                ..Default::default()
            },
        );
        job.wrap = TextWrapping {
            max_width: self.width,
            ..Default::default()
        };
        job
    }

    fn message_to_galley(&self, fonts: &Fonts, msg: &ChatMessage) -> Arc<Galley> {
        fonts.layout_job(self.message_to_layout_job(msg))
    }

    fn convert_new_messages(&mut self, egui_ctx: &Context) {
        let _ = MappedRwLockReadGuard::map(egui_ctx.fonts(), |fonts| {
            let state = self.state.lock().unwrap();
            while state.messages.len() > self.rendered_messages.len() {
                self.rendered_messages.push(
                    self.message_to_galley(fonts, &state.messages[self.rendered_messages.len()]),
                );
            }
            fonts
        });
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
                        Ok(mut cbstate) => {
                            (*cbstate).messages.push(msg.into());
                        }
                        Err(e) => eprintln!("{:?}", e),
                    }
                    self.proxy.send_event(BotfaceEvent::Nonce);
                }
                _ => (),
            }
        }
    }
}
