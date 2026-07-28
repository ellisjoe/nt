use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IoError: {0}")]
    IoError(#[from] std::io::Error),

    #[error("IpError: {0}")]
    InvalidIpAddr(#[from] std::net::AddrParseError),
}
