use crate::filter::{Filter, Next};
use crate::{Request, Response, Result, Error};
use async_trait::async_trait;

use kv_log_macro::{debug, error, log, Level};
use crate::state::State;

/// A logging filter. Logs all requests at debug level, and logs responses at error, warn or info
/// level depending on the status code (5xx, 4xx, and everything else)
pub struct Log;

fn log_response(method: String, uri: String, resp: &Response) {
    let status = resp.as_ref().status();
    let level = if status.is_server_error() {
        Level::Error
    } else if status.is_client_error() {
        Level::Warn
    } else {
        Level::Info
    };

    log!(level, "response", {
        method: method,
        uri: uri,
        status: status.to_string(),
    });
}

#[async_trait]
impl<S: State> Filter<S> for Log
{
    async fn apply(&self, req: Request<S>, next: Next<'_, S>) -> Result<Response> {
        let method = req.method().to_string();
        let uri = req.uri().to_string();

        debug!("request", {
            method: method,
            uri: uri,
        });

        let result = next.next(req).await;

        match &result {
            Ok(resp) => log_response(method, uri, resp),
            Err(Error::Http(resp)) => log_response(method, uri, resp),
            Err(Error::Internal(err)) => {
                error!("internal server error", {
                    method: method,
                    uri: uri,
                    error: err.to_string(),
                    backtrace: format!("{:?}", err),
                });
            }
        }

        result
    }
}
