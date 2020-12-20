use crate::{Endpoint, Request, Responder};
use hyper::{Method, StatusCode};
use route_recognizer::Params;
use std::collections::HashMap;

type DynEndpoint<S> = dyn Endpoint<S> + Send + Sync + 'static;

type Recogniser<S> = route_recognizer::Router<Box<DynEndpoint<S>>>;

pub(crate) struct Router<S> {
    methods: HashMap<Method, Recogniser<S>>,
}

pub(crate) struct RouteTarget<'a, S>
where
    S: Send + Sync + 'static,
{
    pub(crate) ep: &'a DynEndpoint<S>,
    pub(crate) params: Params,
}

impl<S> Router<S>
where
    S: Send + Sync + 'static,
{
    pub(crate) fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    pub(crate) fn add(
        &mut self,
        method: Method,
        path: &str,
        ep: impl Endpoint<S> + Sync + Send + 'static,
    ) {
        self.methods
            .entry(method)
            .or_insert_with(|| route_recognizer::Router::new())
            .add(path, Box::new(ep))
    }

    pub(crate) fn lookup(&self, method: &Method, path: &str) -> RouteTarget<S> {
        match self.methods.get(method) {
            None => RouteTarget {
                ep: &method_not_allowed,
                params: Params::new(),
            },
            Some(recog) => match recog.recognize(path) {
                Ok(match_) => {
                    RouteTarget {
                        ep: &***match_.handler(),
                        params: match_.params().clone(), // TODO - avoid this clone?
                    }
                }
                Err(_) => RouteTarget {
                    ep: &not_found,
                    params: Params::new(),
                },
            },
        }
    }
}

async fn method_not_allowed<S: Sync + 'static>(_: Request<S>) -> impl Responder {
    StatusCode::METHOD_NOT_ALLOWED
}

async fn not_found<S: Sync + 'static>(_: Request<S>) -> impl Responder {
    StatusCode::NOT_FOUND
}
