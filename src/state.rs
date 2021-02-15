
/// State must be implemented for any type being used as the App's state
///
/// State is shared by all requests
pub trait State: Send + Sync + 'static {
    type Context: Send;
    fn new_context(&self) -> Self::Context;
}

/// implement state for () to allow quick examples that don't need it
impl State for () {
    type Context = ();

    fn new_context(&self) -> Self::Context {
        ()
    }
}
