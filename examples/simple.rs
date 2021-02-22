use highnoon::{App, Json, Message, Request, Result, Response, Error};
use hyper::StatusCode;
use serde_derive::Serialize;
use tokio;
use highnoon::filter::Next;
use highnoon::filter::session;
use highnoon::filter::session::{Session, HasSession};
use headers::authorization::{Authorization, Bearer};

#[derive(Default)]
struct State;

#[derive(Default)]
struct Context {
    session: session::Session,
    token: Option<String>
}

impl highnoon::State for State {
    type Context = Context;

    fn new_context(&self) -> Context {
        Context::default()
    }
}

impl session::HasSession for Context {
    fn session(&mut self) -> &mut Session {
        &mut self.session
    }
}


#[derive(Serialize)]
struct Sample {
    data: String,
    value: i32,
}

struct AuthCheck;

#[async_trait::async_trait]
impl highnoon::filter::Filter<State> for AuthCheck {
    async fn apply(&self, mut req: Request<State>, next: Next<'_, State>) -> Result<Response> {
        let auth = req.header::<Authorization<Bearer>>();

        match auth {
            None => return Ok(Response::status(StatusCode::UNAUTHORIZED)),
            Some(bearer) => {
                log::info!("got bearer token: {}", bearer.0.token());
                req.context_mut().token = Some(bearer.0.token().to_owned());
                next.next(req).await
            }
        }
    }
}

fn error_example(req: &Request<State>) -> Result<()> {
    let fail = req.param("fail")?.parse::<bool>()?;

    if fail {
        Err(Error::http((StatusCode::BAD_REQUEST, "you asked for it")))
    } else {
        Ok(())
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    femme::with_level(femme::LevelFilter::Debug);

    let mut app = App::new(State::default());

    app.with(highnoon::filter::Log);
    let memstore = highnoon::filter::session::MemorySessionStore::new();
    app.with(highnoon::filter::session::SessionFilter::new(memstore)
        .with_cookie_name("simple_sid"));

    app.at("/hello")
        .get(|_req| async { "Hello world!\n\n" })
        .post(|mut req: Request<State>| async move {
            let bytes = req.body_bytes().await?;
            Ok(bytes)
        });

    app.at("/echo/:name").get(|mut req: Request<State>| async move {
        let seen = match req.session().get("seen") {
            None => 0,
            Some(s) => s.parse()?
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

    app.at("/json").get(|_req| async {
        Json(Sample {
            data: "hello".to_owned(),
            value: 1234,
        })
    });

    app.at("/error/:fail").get(|req| async move {
        error_example(&req)?;
        Ok("")
    });

    app.at("/query").get(echo_stuff);

    app.at("/ws").ws(|mut ws| async move {
        println!("running the websocket");

        while let Some(msg) = ws.recv().await? {
            println!("message: {}", msg);
            let reply = Message::text("Hello from Highnoon!");
            ws.send(reply).await?;
        }

        Ok(())
    });

    let mut api = App::new(State::default());
    api.with(AuthCheck);

    api.at("check").get(|req: Request<State>| async move {
        println!("URI: {}", req.uri());
        println!("Bearer: {:?}", req.context().token);
        StatusCode::OK
    });
    api.at("user/:name").get(|req: Request<_>| async move {
        println!("URI: {}", req.uri());
        println!("params: {:?}", req.params());
        StatusCode::OK
    });

    app.at("/api/:version").mount(api);

    app.at("/static/*").static_files("examples/resources/");

    app.listen("0.0.0.0:8888").await?;
    Ok(())
}

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
