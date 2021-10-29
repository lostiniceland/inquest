use std::error::Error;
use std::string::FromUtf8Error;

use base64::DecodeError;
use block_modes::BlockModeError;
use thiserror::Error;

use crate::ProbeReport;

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

    #[error("Failure during probe execution!")]
    FailedExecutionError {
        probe_identifier: String,
        source: Box<dyn Error + 'static + Send + Sync>, // additional types needed for thread-safety
                                                        // diagnostics: DiagnosticReport,
    },

    #[error("Failure during assertion execution!")]
    FailedAssertionError {
        probe_identifier: String,
        source: Box<dyn Error + 'static + Send + Sync>, // additional types needed for thread-safety
    },

    #[error("Probe execution failed, due to unmatched assertions")]
    AssertionMatchingError(ProbeReport),

    #[error(transparent)]
    CryptoError(#[from] DecodeError),

    #[error("Key must consist of 10-32 characters but was {length}!")]
    BadCryptoKeyError { length: usize },

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

pub struct DiagnosticReport {}
