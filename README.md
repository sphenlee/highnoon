Highnoon
========

A minimal web framework built on Hyper

**This is a very early development release.
While I'm pretty happy with the API so far, anything could change.**

To get started first implement the `State` trait which holds all data shared by
all the route handlers. This trait contains a single method to get a new
`Context` which is the data shared for the duration of a single request. 
`Context` is generally used for sharing data between filters.

    struct MyState;

    impl highnoon::State for MyState {
        type Context = ();
    
        fn new_context(&self) -> Context {
            ()
        }
    }

Then create an `App` with your `State`, attach filters and routes
and launch the server.

    #[tokio::main]
    async fn main() -> highnoon::Result<()> {
        let mut app = highnoon::App::new(MyState);

        app.with(highnoon::filter::Log);

        app.at("/hello").get(|_request| async {
            "Hello world!\n\n"
        });

        app.listen("0.0.0.0:8888").await?;
        Ok(())
    }

