/// A wrapper over `hyper::Response` with better ergonomics
///
/// ```
/// use highnoon::{Request, Responder, Response};
/// fn example(_: Request<()>) -> impl Responder {
///     Response::ok().json(MyData{...})
/// }
/// ```
use crate::Result;
use headers::{Header, HeaderMapExt};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Body, StatusCode};
use serde::Serialize;
use tokio::io::{AsyncRead, reader_stream};

pub struct Response {
    inner: hyper::Response<Body>,
}

impl Response {
    /// Create an empty response with status code OK (200)
    pub fn ok() -> Self {
        Self {
            inner: hyper::Response::builder()
                .status(StatusCode::OK)
                .body(Body::empty())
                .expect("ok status with empty body should never fail"),
        }
    }

    /// Create an empty response with the given status code
    pub fn status(s: StatusCode) -> Self {
        Self {
            inner: hyper::Response::builder()
                .status(s)
                .body(Body::empty())
                .expect("status with empty body should never fail"),
        }
    }

    /// Set the status code of a response
    pub fn set_status(&mut self, s: StatusCode) {
        *self.inner.status_mut() = s;
    }

    /// Set the body of the response
    pub fn body(mut self, body: impl Into<Body>) -> Self {
        *self.inner.body_mut() = body.into();
        self
    }

    // Set the body to an AsyncRead object
    pub fn reader(mut self, r: impl AsyncRead + Send + 'static) -> Self {
        let body = Body::wrap_stream(reader_stream(r));
        *self.inner.body_mut() = body;
        self
    }

    /// Set the body of the response to a JSON payload
    pub fn json(mut self, body: impl Serialize) -> Result<Self> {
        let data = serde_json::to_vec(&body)?;
        self.set_header(headers::ContentType::json());
        *self.inner.body_mut() = Body::from(data);
        Ok(self)
    }

    /// Set the body of the response to form data
    pub fn form(mut self, body: impl Serialize) -> Result<Self> {
        let form = serde_urlencoded::to_string(body)?;
        self.set_header(headers::ContentType::form_url_encoded());
        *self.inner.body_mut() = Body::from(form);
        Ok(self)
    }

    /// Set a header (from the `headers` crate)
    pub fn header<H: Header>(mut self, h: H) -> Self {
        self.set_header(h);
        self
    }

    pub fn set_header<H: Header>(&mut self, h: H) {
        self.inner.headers_mut().typed_insert(h);
    }

    /// Set a raw header (from the `http` crate)
    pub fn raw_header(mut self, name: impl Into<HeaderName>, key: impl Into<HeaderValue>) -> Self {
        self.inner.headers_mut().insert(name.into(), key.into());
        self
    }

    /// Consume this response and return the inner `hyper::Response`
    pub fn into_inner(self) -> hyper::Response<hyper::Body> {
        self.inner
    }
}

impl From<hyper::Response<Body>> for Response {
    fn from(hyper_response: hyper::Response<Body>) -> Self {
        Self {
            inner: hyper_response,
        }
    }
}
