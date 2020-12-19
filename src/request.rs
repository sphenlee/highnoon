use crate::Result;
use hyper;
use hyper::body::Bytes;
use hyper::Body;
use hyperx::header::{StandardHeader, TypedHeaders};
use std::sync::Arc;

pub struct Request<S: Sync + 'static> {
    state: Arc<S>,
    inner: hyper::Request<Body>,
}

impl<S: Sync + 'static> Request<S> {
    pub(crate) fn new(state: Arc<S>, inner: hyper::Request<Body>) -> Self {
        Self { state, inner }
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

    pub async fn bytes(&mut self) -> Result<Bytes> {
        let bytes = hyper::body::to_bytes(self.inner.body_mut()).await?;
        Ok(bytes)
    }
}
