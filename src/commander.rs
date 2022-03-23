use twitch_irc::message::ServerMessage;
use unicode_segmentation::UWordBounds;
use unicode_segmentation::UnicodeSegmentation;

use crate::irc::{ChatMessage, ComponentMessage, MessageDispatcher};

pub trait IrcCommander {
    fn handle_msg(&mut self, cmd: &str, args: UWordBounds) -> Option<Vec<String>>;
    fn get_commands(&self) -> Vec<String>;
}

pub struct HardCodedCommander {
    static_commands_file: String,
}

impl HardCodedCommander {
    pub fn new(static_commands_file: String) -> Self {
        Self {
            static_commands_file,
        }
    }
}

impl IrcCommander for HardCodedCommander {
    fn get_commands(&self) -> Vec<String> {
        vec!["meow", "project", "shoutout", "so", "woof"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn handle_msg(&mut self, cmd: &str, words: UWordBounds) -> Option<Vec<String>> {
        match cmd {
            "project" => Some(vec![String::from(
                "https://github.com/waynr/twitchy-mcbotface",
            )]),
            "meow" => Some(vec![String::from("woof")]),
            "woof" => Some(vec![String::from("meow")]),
            "so" | "shoutout" => Some(
                words
                    .filter(|&word| word != " ")
                    .map(|word| format!("https://twitch.tv/{}", word).to_string())
                    .collect(),
            ),
            _ => return None,
        }
    }
}

pub struct CommanderComposer {
    commanders: Vec<Box<dyn IrcCommander>>,
    dispatcher: MessageDispatcher,
}

impl CommanderComposer {
    pub fn new(dispatcher: MessageDispatcher, commanders: Vec<Box<dyn IrcCommander>>) -> Self {
        Self {
            commanders,
            dispatcher,
        }
    }

    pub async fn run_commanders(&mut self) {
        while let Ok(message) = self.dispatcher.receiver.recv().await {
            match message {
                ServerMessage::Privmsg(msg) => {
                    // !<botccmd> <arguments>
                    let mut words = msg.message_text.as_str().split_word_bounds();
                    if words.next() != Some("!") {
                        continue;
                    }
                    if let Some(command) = words.next() {
                        match command {
                            "help" => {
                                let mut commands: Vec<String> = self
                                    .commanders
                                    .iter()
                                    .map(|commander| commander.get_commands())
                                    .flatten()
                                    .collect();
                                commands.sort();
                                self.send_msg(&msg.channel_login, &commands.join(", "));
                            }
                            _ => (),
                        }
                        for commander in self.commanders.iter_mut() {
                            if let Some(component_messages) =
                                commander.handle_msg(command, words.clone())
                            {
                                for message in component_messages.iter() {
                                    self.send_msg(&msg.channel_login, message);
                                }
                                break;
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
    }

    pub fn send_msg(&self, channel: &str, message: &str) {
        match self
            .dispatcher
            .sender
            .send(ComponentMessage::Chat(ChatMessage {
                channel: channel.to_string(),
                message: message.to_string(),
            })) {
            Err(e) => println!("failed to send message to channel: {}", e),
            _ => (),
        }
    }
}
