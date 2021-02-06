use crate::router::{RouteTarget, Router};
use crate::static_files::StaticFiles;
use crate::ws::WebSocket;
use crate::{Endpoint, Responder};
use crate::{Request, Result};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method};
use log::info;
use std::convert::Infallible;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::ToSocketAddrs;

pub struct App<S> {
    state: S,
    routes: Router<S>,
}

pub struct Route<'a, 'p, S>
where
    S: Send + Sync + 'static,
{
    path: &'p str,
    app: &'a mut App<S>,
}

impl<'a, 'p, S> Route<'a, 'p, S>
where
    S: Send + Sync + 'static,
{
    pub fn method(self, method: Method, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.app.routes.add(method, self.path, ep);
        self
    }

    pub fn all(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.app.routes.add_all(self.path, ep);
        self
    }

    pub fn get(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.method(Method::GET, ep)
    }

    pub fn post(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.method(Method::POST, ep)
    }

    pub fn put(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.method(Method::PUT, ep)
    }

    pub fn delete(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.method(Method::DELETE, ep)
    }

    pub fn static_files(self, root: impl Into<PathBuf>) -> Self {
        let prefix = self.path.to_owned(); // TODO - borrow issue here
        self.method(Method::GET, StaticFiles::new(root, prefix))
    }

    /*pub fn mount(self, _app: App<S>) -> Self {
        self
    }*/

    pub fn ws<H, F>(self, handler: H)
    where
        H: Send + Sync + 'static + Fn(WebSocket) -> F,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.method(Method::GET, crate::ws::endpoint(handler));
    }
}

impl<S> App<S>
where
    S: Send + Sync + 'static,
{
    pub fn new(state: S) -> Self {
        Self {
            state,
            routes: Router::new(),
        }
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn at<'a, 'p>(&'a mut self, path: &'p str) -> Route<'a, 'p, S> {
        Route { path, app: self }
    }

    pub async fn listen(self, host: impl ToSocketAddrs) -> anyhow::Result<()> {
        let app = Arc::new(self);

        let mut addrs = tokio::net::lookup_host(host).await?;
        let addr = addrs
            .next()
            .ok_or_else(|| anyhow::Error::msg("host lookup returned no hosts"))?;

        let server = hyper::Server::bind(&addr);

        let make_svc = make_service_fn(|addr_stream: &AddrStream| {
            let app = Arc::clone(&app);
            let addr = addr_stream.remote_addr();

            async move {
                Ok::<_, Infallible>(service_fn(move |req: hyper::Request<Body>| {
                    let app = Arc::clone(&app);

                    async move {
                        let RouteTarget { ep, params } =
                            app.routes.lookup(req.method(), req.uri().path());
                        let req = Request::new(Arc::clone(&app), req, params, addr);
                        ep.call(req)
                            .await
                            .or_else(|err| err.into_response())
                            .map(|resp| resp.into_inner())
                            .map_err(|err| err.into_std())
                    }
                }))
            }
        });

        info!("server listening on {}", addr);
        server.serve(make_svc).await?;
        Ok(())
    }
}
