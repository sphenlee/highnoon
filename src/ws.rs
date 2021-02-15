use crate::{Request, Response, Result};
use crate::endpoint::Endpoint;
use async_trait::async_trait;
use futures_util::{SinkExt, TryStreamExt};
use hyper::upgrade::Upgraded;
use hyper::StatusCode;
use log::trace;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use crate::state::State;

/// An endpoint for accepting a websocket connection.
/// Typically constructed by the `Route::ws` method.
#[derive(Debug)]
pub struct WsEndpoint<H, F, S>
where
    S: Send + Sync + 'static,
    H: Send + Sync + 'static + Fn(WebSocket) -> F,
    F: Future<Output = Result<()>> + Send + 'static,
{
    handler: Arc<H>,
    _phantoms: PhantomData<S>,
}

/// Create a websocket endpoint.
/// Typically called by the `Route::ws` method.
pub fn endpoint<H, F, S>(handler: H) -> WsEndpoint<H, F, S>
where
    S: Send + Sync + 'static,
    H: Send + Sync + 'static + Fn(WebSocket) -> F,
    F: Future<Output = Result<()>> + Send + 'static,
{
    WsEndpoint {
        handler: Arc::new(handler),
        _phantoms: PhantomData,
    }
}

#[async_trait]
impl<H, F, S> Endpoint<S> for WsEndpoint<H, F, S>
where
    S: State,
    H: Send + Sync + 'static + Fn(WebSocket) -> F,
    F: Future<Output = Result<()>> + Send + 'static,
{
    async fn call(&self, req: Request<S>) -> Result<Response> {
        let handler = self.handler.clone();

        let res = upgrade_connection(req, handler).await;

        Ok(res)
    }
}

async fn upgrade_connection<S, H, F>(req: Request<S>, handler: Arc<H>) -> Response
where
    S: State,
    H: Send + Sync + 'static + Fn(WebSocket) -> F,
    F: Future<Output = Result<()>> + Send + 'static,
{
    // TODO - check various headers

    if let Some(conn) = req.header::<headers::Connection>() {
        if !conn.contains(hyper::header::UPGRADE) {
            return Response::status(StatusCode::BAD_REQUEST);
        }
    } else {
        return Response::status(StatusCode::BAD_REQUEST);
    }

    if let Some(upgrade) = req.header::<headers::Upgrade>() {
        if upgrade != headers::Upgrade::websocket() {
            return Response::status(StatusCode::BAD_REQUEST);
        }
    } else {
        return Response::status(StatusCode::BAD_REQUEST);
    }

    let key = match req.header::<headers::SecWebsocketKey>() {
        Some(header) => header,
        None => return Response::status(StatusCode::BAD_REQUEST),
    };

    let res = Response::status(StatusCode::SWITCHING_PROTOCOLS)
        .header(headers::Upgrade::websocket())
        .header(headers::Connection::upgrade())
        .header(headers::SecWebsocketAccept::from(key));

    trace!("upgrading connection to websocket");

    tokio::spawn(async move {
        let upgraded = hyper::upgrade::on(req.into_inner())
            .await
            .expect("websocket upgrade failed - TODO report this error");

        let ws = WebSocketStream::from_raw_socket(
            upgraded,
            tokio_tungstenite::tungstenite::protocol::Role::Server,
            None,
        )
        .await;

        let _ = (handler)(WebSocket { inner: ws }).await;
    });

    res
}

/// A websocket connection
pub struct WebSocket {
    inner: WebSocketStream<Upgraded>,
}

impl WebSocket {
    /// Receive a message from the websocket
    pub async fn recv(&mut self) -> Result<Option<Message>> {
        let msg = self.inner.try_next().await?;
        Ok(msg)
    }

    /// Send a message over the websocket
    pub async fn send(&mut self, msg: Message) -> Result<()> {
        self.inner.send(msg).await?;
        Ok(())
    }
}
