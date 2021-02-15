use crate::filter::{Filter, Next};
use crate::{Request, Response, Result};

use async_trait::async_trait;
use kv_log_macro::debug;
use crate::state::State;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use headers::{Header, SetCookie};
use cookie::Cookie;

/// Trait for session storage
#[async_trait]
pub trait SessionStore {
    async fn get(&self, id: &str) -> Option<&str>;
    async fn set(&mut self, id: String, value: String);
    async fn clear(&mut self, id: &str);
}

#[derive(Default)]
pub struct MemorySessionStore {
    data: HashMap<String, String>,
}

impl MemorySessionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionStore for MemorySessionStore {
    async fn get<'s>(&'s self, id: &str) -> Option<&'s str> {
        debug!("memory store get", {
            id: id
        });
        self.data.get(id).map(AsRef::as_ref)
    }

    async fn set(&mut self, id: String, value: String) {
        debug!("memory store set", {
            id: id,
            value: value
        });
        self.data.insert(id, value);
    }

    async fn clear(&mut self, id: &str) {
        debug!("memory store clear", {
            id: id
        });
        self.data.remove(id);
    }
}

/// A filter to extract request cookies and supply response cookies
pub struct SessionFilter {
    store: AsyncMutex<Box<dyn SessionStore + Send + Sync + 'static>>,
}

impl SessionFilter {
    pub fn new(store: impl SessionStore + Send + Sync + 'static) -> SessionFilter {
        SessionFilter {
            store: AsyncMutex::new(Box::new(store))
        }
    }
}

/// A session
#[derive(Default)]
pub struct Session {
    modified: AtomicBool,
    data: Mutex<HashMap<String, String>>,
}

impl Session {
    pub fn get(&self, key: &str) -> Option<String> {
        debug!("session get", {
            key: key
        });
        let data = self.data.lock().unwrap();
        data.get(key).cloned()
    }

    pub fn set(&self, key: String, value: String) {
        debug!("session set", {
            key: key,
            value: value,
        });
        self.data.lock().unwrap().insert(key, value);
        self.modified.store(true, Ordering::Relaxed);
    }

    pub fn is_modified(&self) -> bool {
        self.modified.load(Ordering::Relaxed)
    }

    fn load(&self, data: HashMap<String, String>) {
        *self.data.lock().unwrap() = data;

        // we just loaded fresh data into the session, so clear modified flag to
        // detect if any changes are made that need to be saved back to storage
        self.modified.store(false, Ordering::Relaxed);
    }
}

pub trait HasSession {
    fn session(&mut self) -> Arc<Session>;
}

impl<S> HasSession for Request<S>
where
    S: State,
    S::Context: HasSession
{
    fn session(&mut self) -> Arc<Session> {
        self.context_mut().session()
    }
}

#[async_trait]
impl<S> Filter<S> for SessionFilter
where
    S: State,
    S::Context: HasSession
{
    async fn apply(&self, mut req: Request<S>, next: Next<'_, S>) -> Result<Response> {
        let session = req.session();

        // TODO - pick the cookie name
        let maybe_sid = req
            .cookies()?
            .get("sid")
            .map(|c| c.value().to_owned());

        let sid = if let Some(sid) = maybe_sid {
            debug!("request has session cookie", {
                sid: sid
            });

            let store = self.store.lock().await;
            let raw_data = store.get(&sid).await.unwrap_or("");
            let data= serde_urlencoded::from_str(raw_data)?;
            session.load(data);
            sid
        } else {
            debug!("request has no session cookie");
            "cookie!".to_owned()
        };

        let mut resp = next.next(req).await?;

        if session.is_modified() {
            debug!("session was modified");

            let mut store = self.store.lock().await;
            let raw_data = {
                let data = session.data.lock().unwrap();
                serde_urlencoded::to_string(&*data)?
            };

            // TODO expires etc..
            let cookie = Cookie::new("sid", &sid).to_string();
            let header = headers::HeaderValue::from_str(&cookie)?;
            resp.set_header(SetCookie::decode(&mut vec![&header].into_iter())?);

            store.set(sid, raw_data).await;
        }

        Ok(resp)
    }
}
