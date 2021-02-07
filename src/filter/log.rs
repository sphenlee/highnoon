use crate::filter::{Filter, Next};
use crate::{Request, Response, Result, Error};
use async_trait::async_trait;

use log::{info, debug, warn, error};

/// A logging filter. Logs all requests at debug level, and logs responses at error, warn or info
/// level depending on the status code (5xx, 4xx, and everything else)
pub struct Log;

fn log_response(resp: &Response) {
    let status = resp.as_ref().status();
    if status.is_server_error() {
        error!("response: {}", status);
    } else if status.is_client_error() {
        warn!("response: {}", status)
    } else {
        info!("response: {}", status);
    }
}

#[async_trait]
impl<S> Filter<S> for Log
where
    S: Send + Sync + 'static
{
    async fn apply(&self, req: Request<S>, next: Next<'_, S>) -> Result<Response> {
        debug!("request: {} {}",
            req.method(),
            req.uri()
        );

        let result = next.next(req).await;

        match &result {
            Ok(resp) => log_response(resp),
            Err(Error::Http(resp)) => log_response(resp),
            Err(Error::Internal(err)) => {
                error!("internal server error: {:?}", err);
            }
        }

        result
    }
}
