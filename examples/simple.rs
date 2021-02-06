use highnoon::{App, Json, Message, Request, Result};
use hyper::StatusCode;
use serde_derive::Serialize;
use tokio;

#[derive(Serialize)]
struct Sample {
    data: String,
    value: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut app = App::new(());

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
