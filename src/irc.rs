use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};

use futures::future::join_all;
use tokio::sync::{broadcast, mpsc};

use crate::error::Result;

pub struct MessageDispatcher {
    // note: this should be an MPSC sender
    pub sender: broadcast::Sender<ComponentMessage>,
    pub receiver: broadcast::Receiver<ServerMessage>,

    server_message_sender: broadcast::Sender<ServerMessage>,
}

impl Clone for MessageDispatcher {
    fn clone(&self) -> Self {
        MessageDispatcher {
            sender: self.sender.clone(),
            receiver: self.server_message_sender.subscribe(),
            server_message_sender: self.server_message_sender.clone(),
        }
    }
}

pub struct IrcCore {
    dispatcher: MessageDispatcher,
    sender: broadcast::Sender<ServerMessage>,
    _receiver: broadcast::Receiver<ComponentMessage>,
}

impl IrcCore {
    pub fn new() -> Self {
        // bot sender, dispatcher receiver
        let (sender, dispatcher_receiver) = broadcast::channel(200);
        // dispatcher sender, bot receiver
        let (dispatcher_sender, receiver) = broadcast::channel(200);

        Self {
            dispatcher: MessageDispatcher {
                sender: dispatcher_sender,
                receiver: dispatcher_receiver,
                server_message_sender: sender.clone(),
            },
            sender,
            _receiver: receiver,
        }
    }

    pub async fn run_irc(
        &mut self,
        irc_config: ClientConfig<StaticLoginCredentials>,
    ) -> Result<()> {
        let (incoming_messages, client) =
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(irc_config);

        // handle messages received from IRC server by broadcasting to all components
        let component_broadcaster = self.sender.clone();
        let server_message_handler_client = client.clone();
        let server_message_handler = tokio::spawn(async move {
            Self::server_message_handler(
                incoming_messages,
                component_broadcaster,
                server_message_handler_client,
            )
            .await;
        });

        // handle messages received from components
        println!(
            "num_dispatcher_receivers: {}",
            self.dispatcher.sender.receiver_count()
        );
        let component_message_receiver = self.dispatcher.sender.subscribe();
        println!(
            "num_dispatcher_receivers: {}",
            self.dispatcher.sender.receiver_count()
        );
        let component_message_handler_client = client.clone();
        let component_message_handler = tokio::spawn(async move {
            Self::component_message_handler(
                component_message_handler_client,
                component_message_receiver,
            )
            .await;
        });

        // keep the tokio executor alive.
        // If you return instead of waiting the background task will exit.
        join_all(vec![server_message_handler, component_message_handler]).await;

        Ok(())
    }

    pub fn get_msg_dispatcher(&self) -> MessageDispatcher {
        self.dispatcher.clone()
    }

    pub async fn server_message_handler(
        mut incoming_messages: mpsc::UnboundedReceiver<ServerMessage>,
        sender: broadcast::Sender<ServerMessage>,
        client: TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>,
    ) {
        while let Some(message) = incoming_messages.recv().await {
            match message {
                ServerMessage::Privmsg(msg) => {
                    println!(
                        "[{}] {}: {}",
                        msg.channel_login, msg.sender.login, msg.message_text
                    );
                    match sender.send(ServerMessage::Privmsg(msg)) {
                        Err(e) => {
                            println!("failed to broadcast message: {}", e)
                        }
                        _ => (),
                    }
                }
                ServerMessage::Join(msg) => {
                    println!("[{}] JOIN", msg.channel_login);
                    // match client
                    //     .say(
                    //         msg.channel_login.clone(),
                    //         "hello i am tw1tchymcbotface, humble servant of all stream viewers"
                    //             .to_string(),
                    //     )
                    //     .await
                    // {
                    //     Err(e) => {
                    //         println!("failed to send bot intro message to channel: {}", e)
                    //     }
                    //     _ => (),
                    // }
                }
                _ => (),
            }
        }
    }

    pub async fn component_message_handler(
        client: TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>,
        mut receiver: broadcast::Receiver<ComponentMessage>,
    ) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
        while let Ok(message) = receiver.recv().await {
            interval.tick().await;
            match message {
                ComponentMessage::JoinChannel(msg) => match client.join(msg.channel) {
                    Err(e) => {
                        println!("failed to join requested channel: {}", e);
                    }
                    _ => (),
                },
                ComponentMessage::Chat(msg) => {
                    match client.say(msg.channel.clone(), msg.message.clone()).await {
                        Err(e) => println!(
                            "failed to send message {} to {}: {}",
                            msg.message, msg.channel, e
                        ),
                        _ => (),
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ComponentMessage {
    Chat(ChatMessage),
    JoinChannel(JoinChannelMessage),
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub channel: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct JoinChannelMessage {
    pub channel: String,
}
