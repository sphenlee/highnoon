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
    let mut app = App::new(());

    app.at("/hello")
        .get(|_req| async { "Hello world!\n\n" })
        .post(|mut req: Request<()>| async move {
            let bytes = req.bytes().await?;
            Ok(bytes.to_vec())
        });

    app.at("/echo/:name").get(|req: Request<()>| async move {
        let p = req.param("name");
        match p {
            None => "Hello anonymous\n\n".to_owned(),
            Some(name) => format!("Hello to {}\n\n", name),
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

    app.listen("0.0.0.0:8888".parse().unwrap()).await?;
    Ok(())
}

async fn echo_stuff(mut req: Request<()>) -> Result<StatusCode> {
    let uri = req.uri();
    println!("URI: {}", uri);

    let method = req.method();
    println!("method: {}", method);

    let headers = req.headers();
    println!("header: {:#?}", headers);

    let body = req.bytes().await?;
    println!("body: {}", String::from_utf8_lossy(&body));

    Ok(StatusCode::OK)
}
