use crate::{Request, Response, Result, State};
use crate::endpoint::Endpoint;
use async_trait::async_trait;
use std::future::Future;

mod log;
pub mod session; // TODO - export the needed bits of this

pub use self::log::Log;

/// Represents either the next Filter in the chain, or the actual endpoint if the chain is
/// empty or completed. Use its `next` method to call the next filter/endpoint if the
/// request should continue to be processed.
pub struct Next<'a, S>
where
    S: Send + Sync + 'static
{
    pub(crate) ep: &'a (dyn Endpoint<S> + Send + Sync),
    pub(crate) rest: &'a [Box<dyn Filter<S> + Send + Sync + 'static>],
}

impl<S: State> Next<'_, S>
{
    /// Call either the next filter in the chain, or the actual endpoint if there are no more
    /// filters. Filters are not required to call next (eg. to return a Forbidden status instead)
    pub async fn next(self, req: Request<S>) -> Result<Response> {
        match self.rest.split_first() {
            Some((head, rest)) => {
                let next = Next { ep: self.ep, rest };
                head.apply(req, next).await
            },
            None => self.ep.call(req).await
        }
    }
}

/// A Filter is a reusable bit of logic which wraps an endpoint to provide pre- and post-processing.
/// Filters can call the `Next` argument to continue processing, or may return early to stop the
/// chain. Filters can be used for logging, authentication, cookie handling and many other uses.
#[async_trait]
pub trait Filter<S: State>
{
    async fn apply(&self, req: Request<S>, next: Next<'_, S>) -> Result<Response>;
}

// implement for async functions
#[async_trait]
impl<S, F, Fut> Filter<S> for F
    where
        S: State,
        F: Send + Sync + 'static + for<'n> Fn(Request<S>, Next<'n, S>) -> Fut,
        Fut: Send + 'static + Future<Output = Result<Response>>,
{
    async fn apply(&self, req: Request<S>, next: Next<'_, S>) -> Result<Response> {
        self(req, next).await
    }
}
