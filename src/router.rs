use crate::endpoint::Endpoint;
use crate::state::State;
use crate::{Request, request, Responder};
use hyper::{Method, StatusCode};
use std::collections::HashMap;
use tracing::trace;

type DynEndpoint<S> = dyn Endpoint<S> + Send + Sync + 'static;

type Matcher<S> = matchit::Node<Box<DynEndpoint<S>>>;

pub(crate) struct Router<S> {
    methods: HashMap<Method, Matcher<S>>,
    all: Matcher<S>,
}

pub(crate) struct RouteTarget<'a, S>
where
    S: Send + Sync + 'static,
{
    pub(crate) ep: &'a DynEndpoint<S>,
    pub(crate) params: request::Params,
}

fn copy_params(params: matchit::Params<'_, '_>) -> request::Params {
    params.iter().map(|(k, v)| (k.to_owned(), v.to_owned())).collect()
}

impl<S: State> Router<S> {
    pub(crate) fn new() -> Self {
        Self {
            methods: HashMap::new(),
            all: Matcher::new(),
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
            .or_insert_with(matchit::Node::new)
            .insert(path, Box::new(ep))
            .expect("error inserting route into matcher")
    }

    pub(crate) fn add_all(&mut self, path: &str, ep: impl Endpoint<S> + Sync + Send + 'static) {
        self.all.insert(path, Box::new(ep)).expect("error inserting route into matcher")
    }

    pub(crate) fn lookup(&self, method: &Method, path: &str) -> RouteTarget<S> {
        if let Some(match_) = self
            .methods
            .get(method)
            .and_then(|matcher| matcher.at(path).ok())
        {
            trace!(?method, ?path, "patch matched specific method handler");
            RouteTarget {
                ep: &**match_.value,
                params: copy_params(match_.params)
            }
        } else if let Ok(match_) = self.all.at(path) {
            trace!(?method, ?path, "patch matched 'all' handler");
            RouteTarget {
                ep: &**match_.value,
                params: copy_params(match_.params)
            }
        } else if self
            .methods
            .iter()
            .filter(|(k, _)| k != method)
            .any(|(_, matcher)| matcher.at(path).is_ok())
        {
            trace!(?method, ?path, "patch matched, but wrong method");
            RouteTarget {
                ep: &method_not_allowed,
                params: request::Params::new(),
            }
        } else {
            trace!(?method, ?path, "patch not matched");
            RouteTarget {
                ep: &not_found,
                params: request::Params::new(),
            }
        }
    }
}

async fn method_not_allowed<S: State>(_: Request<S>) -> impl Responder {
    StatusCode::METHOD_NOT_ALLOWED
}

async fn not_found<S: State>(_: Request<S>) -> impl Responder {
    StatusCode::NOT_FOUND
}
