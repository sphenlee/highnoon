use crate::router::{RouteTarget, Router};
use crate::static_files::StaticFiles;
use crate::state::State;
use crate::ws::WebSocket;
use crate::endpoint::Endpoint;
use crate::{Responder, Request, Result, Response};
use crate::filter::{Filter, Next};
use hyper::server::conn::{AddrStream, AddrIncoming};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method};
use log::info;
use std::convert::Infallible;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::ToSocketAddrs;
use async_trait::async_trait;
use hyper::server::Builder;

/// The main entry point to highnoon. An `App` can be launched as a server
/// or mounted into another `App`.
/// Each `App` has a chain of (Filters)[`Filter`]
/// which are applied to each request.
pub struct App<S: State> {
    state: S,
    routes: Router<S>,
    filters: Vec<Box<dyn Filter<S> + Send + Sync + 'static>>,
}

/// Returned by `App::at` and attaches method handlers to a route.
pub struct Route<'a, 'p, S: State>
{
    path: &'p str,
    app: &'a mut App<S>,
}

impl<'a, 'p, S: State> Route<'a, 'p, S>
{
    /// Attach an endpoint for a specific HTTP method
    pub fn method(self, method: Method, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.app.routes.add(method, self.path, ep);
        self
    }

    /// Attach an endpoint for all HTTP methods. These will be checked only if no
    /// specific endpoint exists for the method.
    pub fn all(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.app.routes.add_all(self.path, ep);
        self
    }

    /// Attach an endpoint for GET requests
    pub fn get(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.method(Method::GET, ep)
    }

    /// Attach an endpoint for POST requests
    pub fn post(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.method(Method::POST, ep)
    }

    /// Attach an endpoint for PUT requests
    pub fn put(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.method(Method::PUT, ep)
    }

    /// Attach an endpoint for DELETE requests
    pub fn delete(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        self.method(Method::DELETE, ep)
    }

    /// Serve static files located in the path `root`. The path should end with a wildcard segment
    /// (ie. `/*`). The wildcard portion of the URL will be appended to `root` to form the full
    /// path. The file extension is used to guess a mime type. Files outside of `root` will return
    /// a FORBIDDEN error code; `..` and `.` path segments are allowed as long as they do not navigate
    /// outside of `root`.
    pub fn static_files(self, root: impl Into<PathBuf>) -> Self {
        let prefix = self.path.to_owned(); // TODO - borrow issue here
        self.method(Method::GET, StaticFiles::new(root, prefix))
    }

    /// Mount an app to handle all requests from this path.
    /// The path may contain parameters and these will be merged into
    /// the parameters from individual paths in the inner `App`
    pub fn mount(&mut self, app: App<S>)
    {
        let path = self.path.to_owned() + "/*-highnoon-path-rest-";
        let route = Route { app: self.app, path: &path };
        route.all(app);
    }

    /// Attach a websocket handler to this route
    pub fn ws<H, F>(self, handler: H)
    where
        H: Send + Sync + 'static + Fn(WebSocket) -> F,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.method(Method::GET, crate::ws::endpoint(handler));
    }
}

impl<S: State> App<S>
{
    /// Create a new `App` with the given state.
    /// State must be `Send + Sync + 'static` because it gets shared by all route handlers.
    /// If you need inner mutability use a `Mutex` or similar.
    pub fn new(state: S) -> Self {
        Self {
            state,
            routes: Router::new(),
            filters: vec![],
        }
    }

    /// Get a reference to this App's state
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Append a filter to the chain. Filters are applied to all endpoints in this app, and are
    /// applied in the order they are registered.
    pub fn with<F>(&mut self, filter: F)
    where
        F: Filter<S> + Send + Sync + 'static
    {
        self.filters.push(Box::new(filter));
    }

    /// Create a route at the given path. Returns a `Route` object on which you can
    /// attach handlers for each HTTP method
    pub fn at<'a, 'p>(&'a mut self, path: &'p str) -> Route<'a, 'p, S> {
        Route { path, app: self }
    }

    /// Start a server listening on the given address (See `ToSocketAddrs` from tokio)
    /// This method only returns if there is an error. (Graceful shutdown is TODO)
    pub async fn listen(self, host: impl ToSocketAddrs) -> anyhow::Result<()> {

        let mut addrs = tokio::net::lookup_host(host).await?;
        let addr = addrs
            .next()
            .ok_or_else(|| anyhow::Error::msg("host lookup returned no hosts"))?;

        let builder = hyper::Server::try_bind(&addr)?;
        self.internal_serve(builder).await
    }

    /// Start a server listening on the provided `TcpListener`
    /// This method only returns if there is an error. (Graceful shutdown is TODO)
    pub async fn listen_on(self, tcp: std::net::TcpListener) -> anyhow::Result<()> {
        let builder = hyper::Server::from_tcp(tcp)?;
        self.internal_serve(builder).await
    }

    async fn internal_serve(self, builder: Builder<AddrIncoming>) -> anyhow::Result<()> {
        let app = Arc::new(self);

        let make_svc = make_service_fn(|addr_stream: &AddrStream| {
            let app = app.clone();
            let addr = addr_stream.remote_addr();

            async move {
                Ok::<_, Infallible>(service_fn(move |req: hyper::Request<Body>| {
                    let app = app.clone();

                    async move {
                        let RouteTarget { ep, params } =
                            app.routes.lookup(req.method(), req.uri().path());

                        let req = Request::new(app.clone(), req, params, addr);

                        let next = Next { ep, rest: &*app.filters };

                        next.next(req)
                            .await
                            .or_else(|err| err.into_response())
                            .map(|resp| resp.into_inner())
                            .map_err(|err| err.into_std())
                    }
                }))
            }
        });

        let server = builder.serve(make_svc);
        info!("server listening on {}", server.local_addr());
        server.await?;
        Ok(())
    }
}

#[async_trait]
impl<S: State> Endpoint<S> for App<S>
{
    async fn call(&self, mut req: Request<S>) -> Result<Response> {
        let path_rest = req.param("-highnoon-path-rest-")?;

        let RouteTarget { ep, params } =
            self.routes.lookup(req.method(), path_rest);

        req.merge_params(params);

        let next = Next { ep, rest: &*self.filters };

        next.next(req).await
    }
}
