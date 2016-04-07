use std::fmt;
use std::io;
use std::result;
use std::error;
use std::str;
use std::sync::mpsc;

use rustc_serialize::json;
use websocket;

use control;
use proto;

#[derive(Debug)]
pub enum Error {
    InvalidEnumError(usize),
    IOError(io::Error),
    JSONEncoderError(json::EncoderError),
    JSONDecoderError(json::DecoderError),
    SendControlRequestError(mpsc::SendError<control::Request>),
    SendProtoResponseError(mpsc::SendError<proto::Response>),
    Utf8Error(str::Utf8Error),
    WebSocketError(websocket::result::WebSocketError),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidEnumError(n) =>
                write!(
                    fmt, "InvalidEnumError: {} is not a valid enum value", n
                ),
            Error::IOError(ref err) =>
                write!(fmt, "IOError: {}", err),
            Error::JSONEncoderError(ref err) =>
                write!(fmt, "JSONEncoderError: {}", err),
            Error::JSONDecoderError(ref err) =>
                write!(fmt, "JSONDecoderError: {}", err),
            Error::SendControlRequestError(ref err) =>
                write!(fmt, "SendControlRequestError: {}", err),
            Error::SendProtoResponseError(ref err) =>
                write!(fmt, "SendProtoResponseError: {}", err),
            Error::Utf8Error(ref err) =>
                write!(fmt, "Utf8Error: {}", err),
            Error::WebSocketError(ref err) =>
                write!(fmt, "WebSocketError: {}", err),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::InvalidEnumError(_)        => "InvalidEnumError",
            Error::IOError(_)                 => "IOError",
            Error::JSONEncoderError(_)        => "JSONEncoderError",
            Error::JSONDecoderError(_)        => "JSONDecoderError",
            Error::SendControlRequestError(_) => "SendControlRequestError",
            Error::SendProtoResponseError(_)  => "SendProtoResponseError",
            Error::Utf8Error(_)               => "Utf8Error",
            Error::WebSocketError(_)          => "WebSocketError",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::InvalidEnumError(_)              => None,
            Error::IOError(ref err)                 => Some(err),
            Error::JSONEncoderError(ref err)        => Some(err),
            Error::JSONDecoderError(ref err)        => Some(err),
            Error::SendControlRequestError(ref err) => Some(err),
            Error::SendProtoResponseError(ref err)  => Some(err),
            Error::Utf8Error(ref err)               => Some(err),
            Error::WebSocketError(ref err)          => Some(err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IOError(err)
    }
}

impl From<json::EncoderError> for Error {
    fn from(err: json::EncoderError) -> Self {
        Error::JSONEncoderError(err)
    }
}

impl From<json::DecoderError> for Error {
    fn from(err: json::DecoderError) -> Self {
        Error::JSONDecoderError(err)
    }
}

impl From<mpsc::SendError<control::Request>> for Error {
    fn from(err: mpsc::SendError<control::Request>) -> Self {
        Error::SendControlRequestError(err)
    }
}

impl From<mpsc::SendError<proto::Response>> for Error {
    fn from(err: mpsc::SendError<proto::Response>) -> Self {
        Error::SendProtoResponseError(err)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(err: str::Utf8Error) -> Self {
        Error::Utf8Error(err)
    }
}

impl From<websocket::result::WebSocketError> for Error {
    fn from(err: websocket::result::WebSocketError) -> Self {
        Error::WebSocketError(err)
    }
}

pub type Result<T> = result::Result<T, Error>;
