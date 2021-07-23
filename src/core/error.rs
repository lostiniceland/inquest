use std::string::FromUtf8Error;

use base64::DecodeError;
use block_modes::BlockModeError;
use thiserror::Error;
use crate::core::ProbeReport;

#[derive(Error, Debug)]
pub enum InquestError {



    /// Represents an empty source. For example, an empty text file being given as input.
    #[error("Source contains no data")]
    EmptySource,

    /// Represents a failure to read from input.
    #[error("Read error")]
    ReadError { source: std::io::Error },

    /// Represents all other cases of `std::io::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    URLError(#[from] url::ParseError),

    #[error(transparent)]
    DecryptionError(#[from] BlockModeError),

    #[error(transparent)]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("Configuration data invalid")]
    ConfigurationError,

    #[error("Hocon configuration invalid!")]
    HoconConfigurationError(#[from] hocon::Error),

    #[error("Probe execution failed")]
    FailedExecutionError,

    #[error("Probe execution failed, due to unmatched assertions")]
    AssertionError(ProbeReport),

    #[error(transparent)]
    PostgresError(#[from] tokio_postgres::Error),

    #[error(transparent)]
    OracleError(#[from] oracle::Error),

    #[error(transparent)]
    CryptoError(#[from] DecodeError),

    #[error("Key must consist of 10-32 characters but was {length}!")]
    BadCryptoKeyError { length: usize },

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}
