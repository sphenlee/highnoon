use crate::router::Router;
use crate::static_files::StaticFiles;
use crate::ws::WebSocket;
use crate::{Endpoint, Responder};
use crate::{Request, Result};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method};
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

pub struct App<S> {
    state: Arc<S>,
    routes: Arc<Router<S>>,
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
        let routes = Arc::get_mut(&mut self.app.routes)
            .expect("cannot add routes once serve has been called");
        routes.add(method, self.path, ep);
        self
    }

    pub fn all(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        let routes = Arc::get_mut(&mut self.app.routes)
            .expect("cannot add routes once serve has been called");
        routes.add_all(self.path, ep);
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
            state: Arc::new(state),
            routes: Arc::new(Router::new()),
        }
    }

    pub fn at<'a, 'p>(&'a mut self, path: &'p str) -> Route<'a, 'p, S> {
        Route { path, app: self }
    }

    pub async fn listen(self, addr: SocketAddr) -> Result<()> {
        let server = hyper::Server::bind(&addr);

        let make_svc = make_service_fn(|_: &AddrStream| {
            let state = Arc::clone(&self.state);
            let routes = Arc::clone(&self.routes);
            async move {
                Ok::<_, Infallible>(service_fn(move |req: hyper::Request<Body>| {
                    let state = Arc::clone(&state);
                    let routes = Arc::clone(&routes);
                    async move {
                        let target = routes.lookup(req.method(), req.uri().path());
                        let req = Request::new(state, req, target.params);
                        target
                            .ep
                            .call(req)
                            .await
                            .or_else(|err| err.into_response())
                            .map(|resp| resp.into_inner())
                            .map_err(|err| err.into_std())
                    }
                }))
            }
        });

        server.serve(make_svc).await?;
        Ok(())
    }
}
