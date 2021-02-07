use crate::{App, Error, Result};
use headers::{Header, HeaderMapExt};
use hyper::header::HeaderValue;
use hyper::{body::Buf, Body, HeaderMap};
use route_recognizer::Params;
use serde::de::DeserializeOwned;
use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;

/// An incoming request
pub struct Request<S: Sync + 'static> {
    app: Arc<App<S>>,
    params: Params,
    inner: hyper::Request<Body>,
    remote_addr: SocketAddr,
}

impl<S: Send + Sync + 'static> Request<S> {
    pub(crate) fn new(
        app: Arc<App<S>>,
        inner: hyper::Request<Body>,
        params: Params,
        remote_addr: SocketAddr,
    ) -> Self {
        Self {
            app,
            inner,
            params,
            remote_addr,
        }
    }

    pub(crate) fn merge_params(&mut self, params: Params) {
        for (k, v) in params.iter() {
            self.params.insert(k.to_owned(), v.to_owned());
        }
    }

    /// Get a reference to the `App`'s state
    pub fn state(&self) -> &S {
        self.app.state()
    }

    /// Get the HTTP method being used by this request
    pub fn method(&self) -> &hyper::Method {
        self.inner.method()
    }

    /// Get the URI that was used for this request
    pub fn uri(&self) -> &hyper::Uri {
        self.inner.uri()
    }

    /// Parse the URI query string into an instance of `T` that derives `Deserialize`.
    ///
    /// (To get the raw query string access it via `req.uri().query()`).
    /// If there is no query string, deserialize an empty string.
    pub fn query<T: DeserializeOwned>(&self) -> Result<T> {
        // if there is no query string we can default to empty string
        // serde_urlencode will work if T has all optional fields
        let q = self.inner.uri().query().unwrap_or("");
        let t = serde_urlencoded::from_str::<T>(q)?;
        Ok(t)
    }

    /// Get a typed header from the request
    /// (See also `headers`)
    pub fn header<T: Header>(&self) -> Option<T> {
        self.inner.headers().typed_get()
    }

    /// Get all headers as a `HeaderMap`
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.inner.headers()
    }

    /// Get a route parameter (eg. `:key` or `*key` segments in the URI path)
    pub fn param(&self, param: &str) -> Result<&str> {
        self.params.find(param).ok_or_else(|| {
            // TODO - clean this up
            Error::Internal(anyhow::Error::msg(format!("parameter {} not found", param)))
        })
    }

    /// Get all route parameters
    pub fn params(&self) -> &Params {
        &self.params
    }

    /// Get the request body as a `hyper::Body`
    pub async fn body_mut(&mut self) -> Result<&mut Body> {
        Ok(self.inner.body_mut())
    }

    pub(crate) fn into_inner(self) -> hyper::Request<Body> {
        self.inner
    }

    /// Get a reader to read the request body
    ///
    /// (This does buffer the whole body into memory (not necessarily contiguous memory).
    /// If you need to protect against malicious clients you should access the body via `body_mut`
    pub async fn reader(&mut self) -> Result<impl Read + '_> {
        let buffer = hyper::body::aggregate(self.inner.body_mut()).await?;
        Ok(buffer.reader())
    }

    /// Get the request body as raw bytes in a `Vec<u8>`
    pub async fn body_bytes(&mut self) -> Result<Vec<u8>> {
        let bytes = hyper::body::to_bytes(self.inner.body_mut()).await?;
        Ok(bytes.to_vec())
    }

    /// Get the request body as UTF-8 data in String
    pub async fn body_string(&mut self) -> Result<String> {
        let bytes = hyper::body::to_bytes(self.inner.body_mut()).await?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }

    /// Get the request body as JSON and deserialise into `T`
    pub async fn body_json<T: DeserializeOwned>(&mut self) -> Result<T> {
        let buffer = hyper::body::aggregate(self.inner.body_mut()).await?;
        let json = serde_json::from_reader(buffer.reader())?;
        Ok(json)
    }

    /// Get the address of the remote peer.
    ///
    /// This method uses the network level address only and hwnce may be incorrect if you are
    /// behind a proxy. (This does *not* check for any `Forwarded` headers etc...)
    pub fn remote_addr(&self) -> &SocketAddr {
        &self.remote_addr
    }
}
