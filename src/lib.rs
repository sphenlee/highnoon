pub use anyhow::Result;

pub use headers;
pub use hyper::{Method, StatusCode};
pub use tokio_tungstenite::tungstenite::Message;

pub mod app;
pub mod endpoint;
pub mod request;
pub mod responder;
pub mod response;
pub mod router;
pub mod ws;
mod static_files;

pub use app::App;
pub use endpoint::Endpoint;
pub use request::Request;
pub use responder::{Json, Responder};
pub use response::Response;
/*
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Form(#[from] serde_urlencoded::ser::Error),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    Headers(#[from] headers::Error),
    #[error(transparent)]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

pub type Result<T> = std::result::Result<T, Error>;

impl<E: StdError> From<E> for Error {
    fn from(stderr: E) -> Self {
        Error::Other(Box::new(stderr))
    }
}
*/
