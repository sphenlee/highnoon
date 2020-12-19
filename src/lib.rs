use thiserror::Error;

pub use hyper::{StatusCode, Method};


pub mod app;
pub mod responder;
pub mod response;
pub mod request;
pub mod endpoint;
pub mod router;

pub use app::App;
pub use endpoint::Endpoint;
pub use response::Response;
pub use responder::{Responder, Json};
pub use request::Request;

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
