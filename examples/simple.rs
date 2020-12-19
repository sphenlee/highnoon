use highnoon::{App, Result, Request, Json};
use tokio;
use serde_derive::Serialize;

#[derive(Serialize)]
struct Sample {
    data: String,
    value: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new(());

    app.at("/hello")
        .get(|_req| async { "Hello world!\n\n"})
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
        Json(Sample{
            data: "hello".to_owned(),
            value: 1234,
        })
    });

    app.listen("0.0.0.0:8888".parse().unwrap()).await?;
    Ok(())
}
