use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("disconnected from Twitch router gRPC server")]
    DisconnectedFromTwitchRouterGRPCServer,

    #[error("type conversion failed")]
    TypeConversionError(#[from] std::num::TryFromIntError),
    #[error("failed to deserialize file")]
    SerdeError(#[from] serde_yaml::Error),
    #[error("failed to open file")]
    IOError(#[from] std::io::Error),
    #[error("failed to initialize twitch irc client")]
    TwitchIRCError(#[from] twitch_irc::validate::Error),
    #[error("failed to join tokio task")]
    TokioJoinError(#[from] tokio::task::JoinError),
    #[error("failed to send twitch irc message")]
    TwitchIRCMessageSendError(#[from] twitch_irc::Error<twitch_irc::SecureTCPTransport, twitch_irc::login::StaticLoginCredentials>),
    #[error("something bad: {0}")]
    SomethingBad(String),

    #[error("failed to receive message")]
    AsyncMessageReceiveError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("failed to decode image data")]
    ImageDecodeError(#[from] image::ImageError),

    #[error("{0}")]
    TracingSubscriberParseError(#[from] tracing_subscriber::filter::ParseError),
}
