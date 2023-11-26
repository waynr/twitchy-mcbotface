use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
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

    #[error("irc message unsupported by router: {0}")]
    IrcMessageUnsupportedByRouter(String),

    #[error("failed to serve grpc service: {0}")]
    TonicTransportError(#[from] tonic::transport::Error),
}
