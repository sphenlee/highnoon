use crate::Result;
use bytes::buf::ext::BufExt;
use hyper;
use hyper::header::HeaderValue;
use hyper::{Body, HeaderMap};
use hyperx::header::{StandardHeader, TypedHeaders};
use route_recognizer::Params;
use serde::de::DeserializeOwned;
use std::io::Read;
use std::sync::Arc;

pub struct Request<S: Sync + 'static> {
    state: Arc<S>,
    params: Params,
    inner: hyper::Request<Body>,
}

impl<S: Sync + 'static> Request<S> {
    pub(crate) fn new(state: Arc<S>, inner: hyper::Request<Body>, params: Params) -> Self {
        Self {
            state,
            inner,
            params,
        }
    }

    pub fn state(&self) -> &S {
        &*self.state
    }

    pub fn method(&self) -> &hyper::Method {
        self.inner.method()
    }

    pub fn uri(&self) -> &hyper::Uri {
        self.inner.uri()
    }

    pub fn header<T: StandardHeader>(&self) -> Result<T> {
        let header = self.inner.headers().decode()?;
        Ok(header)
    }

    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.inner.headers()
    }

    pub fn param(&self, param: &str) -> Option<&str> {
        self.params.find(param)
    }

    pub async fn body_mut(&mut self) -> Result<&mut Body> {
        Ok(self.inner.body_mut())
    }

    pub async fn reader(&mut self) -> Result<impl Read + '_> {
        let buffer = hyper::body::aggregate(self.inner.body_mut()).await?;
        Ok(buffer.reader())
    }

    pub async fn bytes(&mut self) -> Result<Vec<u8>> {
        let bytes = hyper::body::to_bytes(self.inner.body_mut()).await?;
        Ok(bytes.to_vec())
    }

    pub async fn json<T: DeserializeOwned>(&mut self) -> Result<T> {
        let buffer = hyper::body::aggregate(self.inner.body_mut()).await?;
        let json = serde_json::from_reader(buffer.reader())?;
        Ok(json)
    }
}
