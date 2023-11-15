use std::sync::Arc;
use std::sync::Mutex;

use tokio::sync::mpsc;

use crate::egui_ui::Chatbox;
use crate::egui_ui::ChatboxState;
use crate::error::Result;
use crate::ndi::NDIFrameData;

pub enum BotfaceEvent {
    Nonce,
}

pub struct Botface {
    chatbox: Chatbox,
    frame_sender: mpsc::UnboundedSender<NDIFrameData>,
}

impl Botface {
    pub fn new(frame_sender: mpsc::UnboundedSender<NDIFrameData>) -> Result<Self> {
        let chatbox_state = Arc::new(Mutex::new(ChatboxState::new()));
        let chatbox = Chatbox::new(chatbox_state);
        Ok(Self {
            chatbox,
            frame_sender,
        })
    }

    pub fn chatbox_state(&self) -> Arc<Mutex<ChatboxState>> {
        self.chatbox.state()
    }
}
