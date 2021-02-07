use highnoon::{App, Json, Message, Request, Result, Response};
use hyper::StatusCode;
use serde_derive::Serialize;
use tokio;
use highnoon::filter::Next;

#[derive(Serialize)]
struct Sample {
    data: String,
    value: i32,
}

struct AuthCheck;

#[async_trait::async_trait]
impl highnoon::filter::Filter<()> for AuthCheck {
    async fn apply(&self, req: Request<()>, next: Next<'_, ()>) -> Result<Response> {
        let auth = req.header::<headers::Authorization<headers::authorization::Bearer>>();

        match auth {
            None => return Ok(Response::status(StatusCode::UNAUTHORIZED)),
            Some(bearer) => {
                log::info!("got bearer token: {}", bearer.0.token());
                next.next(req).await
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut app = App::new(());

    app.with(highnoon::filter::log::Log);

    app.at("/hello")
        .get(|_req| async { "Hello world!\n\n" })
        .post(|mut req: Request<()>| async move {
            let bytes = req.body_bytes().await?;
            Ok(bytes)
        });

    app.at("/echo/:name").get(|req: Request<()>| async move {
        let p = req.param("name");
        match p {
            Err(_) => "Hello anonymous\n\n".to_owned(),
            Ok(name) => format!("Hello to {}\n\n", name),
        }
    });

    app.at("/json").get(|_req| async {
        Json(Sample {
            data: "hello".to_owned(),
            value: 1234,
        })
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

    let mut api = App::new(());
    api.with(AuthCheck);

    api.at("check").get(|req: Request<_>| async move {
        println!("URI: {}", req.uri());
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

async fn echo_stuff(mut req: Request<()>) -> Result<StatusCode> {
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
