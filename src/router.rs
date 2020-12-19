use crate::{Endpoint, Request, Responder};
use hyper::{Method, StatusCode};
use std::collections::HashMap;
use route_recognizer::Match;

type Recogniser<S> = route_recognizer::Router<Box<dyn Endpoint<S> + Send + Sync + 'static>>;

pub struct Router<S> {
    methods: HashMap<Method, Recogniser<S>>
}

impl<S> Router<S>
    where S: Send + Sync + 'static
{
    pub(crate) fn new() -> Self {
        Self {
            methods: HashMap::new()
        }
    }

    pub(crate) fn add(&mut self, method: Method, path: &str, ep: impl Endpoint<S> + Sync + Send + 'static) {
        self.methods.entry(method)
            .or_insert_with(|| route_recognizer::Router::new())
            .add(path, Box::new(ep))
    }

    pub(crate) fn lookup(&self, method: &Method, path: &str) -> &(dyn Endpoint<S> + Send + Sync + 'static) {
        match self.methods.get(method) {
            None => &method_not_allowed,
            Some(recog) => {
                match recog.recognize(path) {
                    Ok(route_match) => {
                        let ep = *route_match.handler();
                        &**ep
                    },
                    Err(_) => &not_found
                }
            }
        }
    }
}

fn method_not_allowed<S: Sync + 'static>(_: Request<S>) -> impl Responder {
    StatusCode::METHOD_NOT_ALLOWED
}

fn not_found<S: Sync + 'static>(_: Request<S>) -> impl Responder {
    StatusCode::NOT_FOUND
}
