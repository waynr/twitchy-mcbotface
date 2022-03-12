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
}
