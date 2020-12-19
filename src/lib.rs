pub use hyper::{StatusCode, Method};

pub mod app;
pub mod responder;
pub mod response;
pub mod request;

pub use response::Response;
pub use responder::{Responder, Json};
pub use request::Request;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;
