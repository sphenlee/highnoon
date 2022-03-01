use highnoon::{App, Json, Request, StatusCode};
use serde_json::{json, Value};

fn make_app() -> App<()> {
    let mut app = App::new(());

    app.at("/greeting").get(|_req| async { "Hello World!" });

    app.at("/reverse").get(|mut req: Request<()>| async move {
        let mut data = req.body_bytes().await?;
        data.reverse();
        Ok(data)
    });

    app.at("/json").get(|mut req: Request<()>| async move {
        let data: Value = req.body_json().await?;
        let greeting = data
            .get("greeting")
            .and_then(|val| val.as_str())
            .map(|s| s.to_owned());
        Ok(Json(greeting))
    });

    app
}

#[tokio::main]
#[test]
pub async fn test_greeting() -> highnoon::Result<()> {
    let tc = make_app().test();

    let mut resp = tc.get("/greeting").send().await?;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body_string().await?, "Hello World!");

    Ok(())
}

#[tokio::main]
#[test]
pub async fn test_reverse() -> highnoon::Result<()> {
    let tc = make_app().test();

    let mut resp = tc.get("/reverse").body("Hello World!")?.send().await?;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body_string().await?, "!dlroW olleH");

    Ok(())
}

#[tokio::main]
#[test]
pub async fn test_json() -> highnoon::Result<()> {
    let tc = make_app().test();

    let mut resp = tc
        .get("/json")
        .json(json!({
            "greeting": "Hello World!"
        }))?
        .send()
        .await?;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body_string().await?, "\"Hello World!\"");

    Ok(())
}

#[tokio::main]
#[test]
pub async fn test_404() -> highnoon::Result<()> {
    let tc = make_app().test();

    let resp = tc.get("/no_such_route").send().await?;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::main]
#[test]
pub async fn test_method_not_allowed() -> highnoon::Result<()> {
    let tc = make_app().test();

    let resp = tc.delete("/greeting").send().await?;

    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);

    Ok(())
}
