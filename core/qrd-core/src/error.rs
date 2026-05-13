use std::error::Error;
use std::fmt;

/// Result alias used across the QRD core scaffold.
pub type Result<T> = std::result::Result<T, QrdError>;

/// Minimal error taxonomy for the Phase 1 scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrdError {
    InvalidMagic,
    InvalidHeaderLength,
    InvalidReservedField,
    InvalidFooterLength,
    UnsupportedEncoding(u8),
    UnsupportedCompression(u8),
    InvalidSchema(String),
    UnexpectedEof,
    AuthenticationFailed,
    NotImplemented(&'static str),
}

impl fmt::Display for QrdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => f.write_str("invalid QRD magic bytes"),
            Self::InvalidHeaderLength => f.write_str("invalid QRD header length"),
            Self::InvalidReservedField => f.write_str("reserved header field must be zero"),
            Self::InvalidFooterLength => f.write_str("invalid QRD footer length"),
            Self::UnsupportedEncoding(id) => write!(f, "unsupported encoding id: {id:#04x}"),
            Self::UnsupportedCompression(id) => {
                write!(f, "unsupported compression id: {id:#04x}")
            }
            Self::InvalidSchema(message) => write!(f, "invalid schema: {message}"),
            Self::UnexpectedEof => f.write_str("unexpected end of input"),
            Self::AuthenticationFailed => f.write_str("authentication failed"),
            Self::NotImplemented(feature) => write!(f, "feature not implemented: {feature}"),
        }
    }
}

impl Error for QrdError {}
