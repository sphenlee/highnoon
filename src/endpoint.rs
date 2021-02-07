use crate::{Request, Responder, Response, Result};
use async_trait::async_trait;
use std::future::Future;

pub type DynEndpoint<S> = dyn Endpoint<S> + Send + Sync + 'static;

#[async_trait]
pub trait Endpoint<S>
where
    S: Send + Sync + 'static,
{
    async fn call(&self, req: Request<S>) -> Result<Response>;
}

#[async_trait]
impl<S, F, Fut, R> Endpoint<S> for F
where
    F: Send + Sync + 'static + Fn(Request<S>) -> Fut,
    Fut: Future<Output = R> + Send + 'static,
    R: Responder + 'static,
    S: Send + Sync + 'static,
{
    async fn call(&self, req: Request<S>) -> Result<Response> {
        (self)(req).await.into_response()
    }
}
