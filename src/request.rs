use crate::{App, Error, Result};
use headers::{Header, HeaderMapExt};
use hyper::header::HeaderValue;
use hyper::{body::Buf, Body, HeaderMap};
use route_recognizer::Params;
use serde::de::DeserializeOwned;
use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;

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

    pub fn state(&self) -> &S {
        self.app.state()
    }

    pub fn method(&self) -> &hyper::Method {
        self.inner.method()
    }

    pub fn uri(&self) -> &hyper::Uri {
        self.inner.uri()
    }

    pub fn query<T: DeserializeOwned>(&self) -> Result<T> {
        // if there is no query string we can default to empty string
        // serde_urlencode will work if T has all optional fields
        let q = self.inner.uri().query().unwrap_or("");
        let t = serde_urlencoded::from_str::<T>(q)?;
        Ok(t)
    }

    pub fn header<T: Header>(&self) -> Option<T> {
        self.inner.headers().typed_get()
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.inner.headers()
    }

    pub fn param(&self, param: &str) -> Result<&str> {
        self.params.find(param).ok_or_else(|| {
            // TODO - clean this up
            Error::Internal(anyhow::Error::msg(format!("parameter {} not found", param)))
        })
    }

    pub async fn body_mut(&mut self) -> Result<&mut Body> {
        Ok(self.inner.body_mut())
    }

    pub(crate) fn into_inner(self) -> hyper::Request<Body> {
        self.inner
    }

    pub async fn reader(&mut self) -> Result<impl Read + '_> {
        let buffer = hyper::body::aggregate(self.inner.body_mut()).await?;
        Ok(buffer.reader())
    }

    pub async fn body_bytes(&mut self) -> Result<Vec<u8>> {
        let bytes = hyper::body::to_bytes(self.inner.body_mut()).await?;
        Ok(bytes.to_vec())
    }

    pub async fn body_string(&mut self) -> Result<String> {
        let bytes = hyper::body::to_bytes(self.inner.body_mut()).await?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }

    pub async fn body_json<T: DeserializeOwned>(&mut self) -> Result<T> {
        let buffer = hyper::body::aggregate(self.inner.body_mut()).await?;
        let json = serde_json::from_reader(buffer.reader())?;
        Ok(json)
    }

    pub fn remote_addr(&self) -> &SocketAddr {
        &self.remote_addr
    }
}
