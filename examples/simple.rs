use highnoon::{App, Result};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new(());

    app.at("/hello").get(|_req| "Hello world!");

    app.listen("0.0.0.0:8888".parse().unwrap()).await?;
    Ok(())
}
