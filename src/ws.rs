use crate::{Endpoint, Request, Response, Result};
use async_trait::async_trait;
use futures_util::{SinkExt, TryStreamExt};
use headers;
use hyper::upgrade::Upgraded;
use hyper::StatusCode;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

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
    S: Send + Sync + 'static,
    H: Send + Sync + 'static + Fn(WebSocket) -> F,
    F: Future<Output = Result<()>> + Send + 'static,
{
    async fn call(&self, req: Request<S>) -> Result<Response> {
        let handler = self.handler.clone();

        let res = upgrade_connection(req, handler).await?;

        Ok(res)
    }
}

async fn upgrade_connection<S, H, F>(req: Request<S>, handler: Arc<H>) -> Result<Response>
where
    S: Send + Sync + 'static,
    H: Send + Sync + 'static + Fn(WebSocket) -> F,
    F: Future<Output = Result<()>> + Send + 'static,
{
    let key = match req.header::<headers::SecWebsocketKey>() {
        Some(header) => header,
        None => return Ok(Response::status(StatusCode::BAD_REQUEST)),
    };

    let res = Response::status(StatusCode::SWITCHING_PROTOCOLS)
        .header(headers::Upgrade::websocket())
        .header(headers::Connection::upgrade())
        .header(headers::SecWebsocketAccept::from(key));

    println!("upgrading to websocket!");

    tokio::spawn(async move {
        let upgraded = req
            .into_body()
            .on_upgrade()
            .await
            .expect("websocket upgrade failed - TODO report this error");

        println!("starting the websocket protocol");

        let ws = WebSocketStream::from_raw_socket(
            upgraded,
            tokio_tungstenite::tungstenite::protocol::Role::Server,
            None,
        )
        .await;

        println!("calling ws handler");
        let _ = (handler)(WebSocket { inner: ws }).await;
    });

    Ok(res)
}

pub struct WebSocket {
    inner: WebSocketStream<Upgraded>,
}

impl WebSocket {
    pub async fn recv(&mut self) -> Result<Option<Message>> {
        let msg = self.inner.try_next().await?;
        Ok(msg)
    }

    pub async fn send(&mut self, msg: Message) -> Result<()> {
        self.inner.send(msg).await?;
        Ok(())
    }
}
