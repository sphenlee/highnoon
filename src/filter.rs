use crate::{Request, Response, Result};
use crate::endpoint::Endpoint;
use async_trait::async_trait;
use std::future::Future;

pub mod log;

pub use self::log::Log;

pub struct Next<'a, S>
where
    S: Send + Sync + 'static
{
    pub(crate) ep: &'a (dyn Endpoint<S> + Send + Sync),
    pub(crate) rest: &'a [Box<dyn Filter<S> + Send + Sync + 'static>],
}

impl<S> Next<'_, S>
where
    S: Send + Sync + 'static
{
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

#[async_trait]
pub trait Filter<S>
where
    S: Send + Sync + 'static
{
    async fn apply(&self, req: Request<S>, next: Next<'_, S>) -> Result<Response>;
}

// implement for async functions
#[async_trait]
impl<S, F, Fut> Filter<S> for F
    where
        S: Send + Sync + 'static,
        F: Send + Sync + 'static + for<'n> Fn(Request<S>, Next<'n, S>) -> Fut,
        Fut: Send + 'static + Future<Output = Result<Response>>,
{
    async fn apply(&self, req: Request<S>, next: Next<'_, S>) -> Result<Response> {
        self(req, next).await
    }
}
