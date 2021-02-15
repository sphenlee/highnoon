use crate::{Request, Responder, Response, Result};
use async_trait::async_trait;
use std::future::Future;
use crate::state::State;

pub type DynEndpoint<S> = dyn Endpoint<S> + Send + Sync + 'static;

/// Implement `Endpoint` for a type to be used as a method handler.
///
/// It is already implemented for functions of `Request` to `Result<Response>`
/// which is the simplest (and most convenient) kind of handler.
/// You can implement it manually for endpoints that may require some kind of local state.
#[async_trait]
pub trait Endpoint<S: State>
{
    async fn call(&self, req: Request<S>) -> Result<Response>;
}

#[async_trait]
impl<S, F, Fut, R> Endpoint<S> for F
where
    F: Send + Sync + 'static + Fn(Request<S>) -> Fut,
    Fut: Future<Output = R> + Send + 'static,
    R: Responder + 'static,
    S: State,
{
    async fn call(&self, req: Request<S>) -> Result<Response> {
        (self)(req).await.into_response()
    }
}
