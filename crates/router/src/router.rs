use std::net::ToSocketAddrs;
use std::pin::Pin;

use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use super::error::{Error, Result};
use super::irc::{MessageDispatcher, ServerMessage};

use super::pb::{IrcMessage, StreamIrcRequest};
use super::pb::twitch_server::{Twitch, TwitchServer};

#[derive(Debug)]
pub struct TwitchRouter {
    irc_message_dispatcher: MessageDispatcher,
}

impl TwitchRouter {
    pub fn new(irc_message_dispatcher: MessageDispatcher) -> Self {
        Self {
            irc_message_dispatcher,
        }
    }

    pub async fn run(self) -> Result<()> {
        Server::builder()
            .add_service(TwitchServer::new(self))
            .serve("localhost:50551".to_socket_addrs()?.next().unwrap())
            .await?;

        Ok(())
    }
}

type IrcResult<T> = std::result::Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = std::result::Result<IrcMessage, Status>> + Send>>;

#[tonic::async_trait]
impl Twitch for TwitchRouter {
    type StreamIrcMessagesStream = ResponseStream;

    async fn stream_irc_messages(
        &self,
        req: Request<StreamIrcRequest>,
    ) -> IrcResult<Self::StreamIrcMessagesStream> {
        let remote_addr = req.remote_addr();

        tracing::info!("TwitchRouter::stream_irc_messages");
        tracing::info!("\tclient connected from: {:?}", remote_addr);

        let (tx, rx) = mpsc::channel(128);
        let mut dispatcher = self.irc_message_dispatcher.clone();
        tokio::spawn(async move {
            loop {
                match dispatcher.receiver.recv().await {
                    Ok(item) => {
                        let item: IrcMessage = match item.try_into() {
                            Ok(m) => m,
                            Err(e) => {
                                tracing::warn!("unable to convert message: {e}");
                                continue;
                            }
                        };
                        match tx.send(std::result::Result::<_, Status>::Ok(item)).await {
                            Ok(_) => {
                                // server response was queued to be sent to client
                            }
                            Err(e) => {
                                tracing::warn!("error sending response to client: {e:?}");
                                break;
                            }
                        }
                    }
                    Err(RecvError::Closed) => {
                        tracing::warn!("message dispatcher closed gRPC server exiting");
                        break;
                    }
                    Err(RecvError::Lagged(i)) => {
                        tracing::warn!("receiver lagged too far behind, skipped {i} messages");
                    }
                }
            }
            tracing::info!("client ({:?}) disconnected ", remote_addr);
        });
        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::StreamIrcMessagesStream
        ))
    }
}

impl TryFrom<ServerMessage> for IrcMessage {
    type Error = Error;

    fn try_from(sm: ServerMessage) -> Result<Self> {
        match sm {
            ServerMessage::Privmsg(msg) => Ok(Self {
                user: msg.channel_login,
                message: msg.message_text,
            }),
            m => Err(Error::IrcMessageUnsupportedByRouter(format!("{m:?}"))),
        }
    }
}
