use crate::{Result, Request, Response, Responder};
use async_trait::async_trait;

#[async_trait]
pub trait Endpoint<S>
    where S: Send + Sync + 'static
{
    async fn call(&self, req: Request<S>) -> Result<Response>;
}

#[async_trait]
impl<S, F, R> Endpoint<S> for F
    where F: Send + Sync + 'static + Fn(Request<S>) -> R,
    R: Responder + 'static,
    S: Send + Sync + 'static,
{
    async fn call(&self, req: Request<S>) -> Result<Response> {
        (self)(req).into_response()
    }
}
