use crate::{Result, Request};
use crate::Endpoint;
use crate::router::Router;
use std::sync::Arc;
use std::net::SocketAddr;
use hyper::service::{make_service_fn, service_fn};
use hyper::server::conn::AddrStream;
use std::convert::Infallible;
use hyper::{Body, Method};

pub struct App<S> {
    state: Arc<S>,
    routes: Arc<Router<S>>,
}

pub struct Route<'a, 'p, S>
    where S: Send + Sync + 'static
{
    path: &'p str,
    app: &'a mut App<S>,
}

impl<'a, 'p, S> Route<'a, 'p, S>
    where S: Send + Sync + 'static
{
    pub fn get(self, ep: impl Endpoint<S> + Send + Sync + 'static) -> Self {
        let routes = Arc::get_mut(&mut self.app.routes).expect("cannot add routes once serve has been called");
        routes.add(Method::GET, self.path, ep);
        self
    }
}

impl<S> App<S>
    where S: Send + Sync + 'static
{
    pub fn new(state: S) -> Self {
        Self {
            state: Arc::new(state),
            routes: Arc::new(Router::new())
        }
    }

    pub fn at<'a, 'p>(&'a mut self, path: &'p str) -> Route<'a, 'p, S> {
        Route {
            path,
            app: self
        }
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
                        let ep = routes.lookup(req.method(), req.uri().path());
                        let req = Request::new(Arc::clone(&state), req);
                        ep.call(req).await.map(|resp| resp.into_inner())
                    }
                }))
            }
        });

        server.serve(make_svc).await?;
        Ok(())
    }
}