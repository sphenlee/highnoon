use thiserror::Error;

pub use hyper::{Method, StatusCode};

pub mod app;
pub mod endpoint;
pub mod request;
pub mod responder;
pub mod response;
pub mod router;

pub use app::App;
pub use endpoint::Endpoint;
pub use request::Request;
pub use responder::{Json, Responder};
pub use response::Response;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Form(#[from] serde_urlencoded::ser::Error),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    Hyperx(#[from] hyperx::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
