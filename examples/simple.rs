use headers::authorization::{Authorization, Bearer};
use highnoon::filter::session;
use highnoon::filter::session::{HasSession, Session};
use highnoon::filter::Next;
use highnoon::{App, Error, Json, Message, Request, Response, Result};
use hyper::StatusCode;
use serde_derive::Serialize;
use tokio;
use tracing::info;

/// a fake database, in a real server this would be a pool connection
#[derive(Debug)]
struct Db;

impl Default for Db {
    fn default() -> Self {
        Db
    }
}

/// An extension trait to get access to the database
trait HasDb {
    fn get_db(&self) -> &Db;
}

/// Application state
#[derive(Default)]
struct State {
    db: Db,
}

/// Per request context
#[derive(Default)]
struct Context {
    session: session::Session,
}

/// Implement state for our struct
impl highnoon::State for State {
    type Context = Context;

    fn new_context(&self) -> Context {
        Context::default()
    }
}

/// Our context has sessions
impl session::HasSession for Context {
    fn session(&mut self) -> &mut Session {
        &mut self.session
    }
}

/// Our state has a database
impl HasDb for State {
    fn get_db(&self) -> &Db {
        &self.db
    }
}

/// We can also extend the Request for states that have a Db
impl<S> HasDb for Request<S>
where
    S: highnoon::State + HasDb,
{
    fn get_db(&self) -> &Db {
        self.state().get_db()
    }
}

/// Data we store in the Session
#[derive(Serialize)]
struct Sample {
    data: String,
    value: i32,
}


#[derive(Default)]
struct ApiState;

#[derive(Default)]
struct ApiContext {
    token: Option<String>,
}

impl From<Context> for ApiContext {
    fn from(_: Context) -> Self {
        ApiContext::default()
    }
}

/// Implement state for our struct
impl highnoon::State for ApiState {
    type Context = ApiContext;

    fn new_context(&self) -> ApiContext {
        ApiContext::default()
    }
}

/// A filter for checking token auth
struct AuthCheck;

#[async_trait::async_trait]
impl highnoon::filter::Filter<ApiState> for AuthCheck {
    async fn apply(&self, mut req: Request<ApiState>, next: Next<'_, ApiState>) -> Result<Response> {
        let auth = req.header::<Authorization<Bearer>>();

        match auth {
            None => return Ok(Response::status(StatusCode::UNAUTHORIZED)),
            Some(bearer) => {
                info!("got bearer token: {}", bearer.0.token());
                req.context_mut().token = Some(bearer.0.token().to_owned());
                next.next(req).await
            }
        }
    }
}

/// A route handler that returns an Error which translates into HTTP bad request
fn error_example(req: &Request<State>) -> Result<()> {
    let fail = req.param("fail")?.parse::<bool>()?;

    if fail {
        Err(Error::bad_request("you asked for it"))
    } else {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .init();

    // create the root app
    let mut app = App::new(State::default());

    // install the logging filter
    app.with(highnoon::filter::Log);

    // setup session handling
    let memstore = highnoon::filter::session::MemorySessionStore::new();
    app.with(
        highnoon::filter::session::SessionFilter::new(memstore)
            .with_cookie_name("simple_sid")
            .with_expiry(time::Duration::minutes(5))
            .with_callback(|cookie| {
                // for demo purposes - default is secure cookies
                cookie.set_secure(false);
            }),
    );

    // setup routes
    // basic route to show get and post
    app.at("/hello")
        .get(|_req| async { "Hello world!\n\n" })
        .post(|mut req: Request<State>| async move {
            let bytes = req.body_bytes().await?;
            Ok(bytes)
        });

    // a route with a parameter, also uses session data
    app.at("/echo/:name")
        .get(|mut req: Request<State>| async move {
            let seen = match req.session().get("seen") {
                None => 0,
                Some(s) => s.parse()?,
            };

            let greeting = if seen > 1 {
                "You again!"
            } else if seen == 1 {
                "Welcome back"
            } else {
                "Hello"
            };

            req.session().set("seen".to_owned(), (seen + 1).to_string());

            let p = req.param("name");
            Ok(match p {
                Err(_) => format!("{} anonymous\n\n", greeting),
                Ok(name) => format!("{} {}\n\n", greeting, name),
            })
        });

    // route that accesses the "database"
    app.at("/db").get(|req: Request<State>| async move {
        let db = req.get_db();
        format!("database is {:?}", db)
    });

    // return some json
    app.at("/json").get(|_req| async {
        Json(Sample {
            data: "hello".to_owned(),
            value: 1234,
        })
    });

    // demonstrate using Err to return HTTP errors
    app.at("/error/:fail").get(|req| async move {
        error_example(&req)?;
        Ok("")
    });

    // use a function as a handler
    app.at("/query").get(echo_stuff);

    // websocket
    app.at("/ws").ws(|mut tx, mut rx| async move {
        println!("running the websocket");

        while let Some(msg) = rx.recv().await? {
            println!("message: {}", msg);
            let reply = Message::text("Hello from Highnoon!");
            tx.send(reply).await?;
        }

        Ok(())
    });

    // create a sub-app with the auth filter
    let mut api = App::new(ApiState::default());
    api.with(AuthCheck);

    // check auth is working
    api.at("/check").get(|req: Request<ApiState>| async move {
        println!("URI: {}", req.uri());
        println!("Bearer: {:?}", req.context().token);
        StatusCode::OK
    });
    // check that parameters get merged
    api.at("/user/:name").get(|req: Request<_>| async move {
        println!("URI: {}", req.uri());
        println!("params: {:?}", req.params());
        StatusCode::OK
    });

    // mount the sub-app into the root
    app.at("/api/:version").mount(api);

    // static files handling
    app.at("/static/*path").static_files("examples/resources/");

    // launch the server!
    app.listen("0.0.0.0:8888").await?;
    Ok(())
}

/// demonstrate all the request methods
async fn echo_stuff(mut req: Request<State>) -> Result<StatusCode> {
    let uri = req.uri();
    println!("URI: {}", uri);

    let method = req.method();
    println!("method: {}", method);

    let headers = req.headers();
    println!("header: {:#?}", headers);

    let body = req.body_bytes().await?;
    println!("body: {}", String::from_utf8_lossy(&body));

    println!("remote addr: {}", req.remote_addr());

    Ok(StatusCode::OK)
}
