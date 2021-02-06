use crate::filter::{Filter, Next};
use crate::{Request, Response, Result};
use async_trait::async_trait;

use log::info;

pub struct Log;

#[async_trait]
impl<S> Filter<S> for Log
where
    S: Send + Sync + 'static
{
    async fn apply(&self, req: Request<S>, next: Next<'_, S>) -> Result<Response> {
        info!("request: {} {}",
            req.method(),
            req.uri()
        );
        let resp = next.next(req).await?;
        info!("response: {}", resp.get_status());
        Ok(resp)
    }
}
