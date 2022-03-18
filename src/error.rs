use thiserror::Error;
use ndi_sdk::SendCreateError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("ndi sdk error: {0}")]
    NDISDKError(String),
    #[error("ndi sdk error: {0}")]
    NDISDKSenderCreatError(#[from] SendCreateError),
    #[error("invalid SDI byte capacity")]
    InvalidSDIByteBufferCapacity,
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
}
