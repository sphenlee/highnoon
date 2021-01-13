use crate::{Responder, Response, Result};
use hyper::StatusCode;
use std::error::Error as StdError;
use std::fmt::Formatter;

pub enum Error {
    /// An error that should get returned to the client
    Http(Response),
    /// Internal errors, reported as 500 Internal Server Error and logged locally
    Internal(anyhow::Error),
}

impl Error {
    pub fn into_std(self) -> Box<dyn StdError + Send + Sync + 'static> {
        match self {
            Error::Http(_) => panic!("http error??!"),
            Error::Internal(err) => err.into(),
        }
    }
}

impl Responder for Error {
    fn into_response(self) -> Result<Response> {
        match self {
            Error::Http(resp) => Ok(resp),
            Error::Internal(_err) => Ok(Response::status(StatusCode::INTERNAL_SERVER_ERROR)),
        }
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(e: E) -> Self {
        Error::Internal(anyhow::Error::new(e))
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Internal(err) => f
                .debug_struct("Error::Internal")
                .field("inner", err)
                .finish(),
            Error::Http(resp) => f.debug_struct("Error::Http").field("inner", resp).finish(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Internal(err) => write!(f, "Internal Error: {:?}", err),
            Error::Http(resp) => write!(f, "{:?}", resp),
        }
    }
}
