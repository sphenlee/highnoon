/// State must be implemented for any type being used as the App's state
///
/// State is shared by all requests, and must be safe to be shared between
/// threads (Send + Sync + 'static)
///
/// The state also creates the Context objects used to store request local
/// data.
/// Before processing a request a new context is created
pub trait State: Send + Sync + 'static {
    /// Type of the request local context
    type Context: Send + Sync + 'static;

    /// Creating a new Context to be used for a single request
    fn new_context(&self) -> Self::Context;
}

/// Implement state for void
impl State for () {
    type Context = ();

    fn new_context(&self) -> Self::Context {}
}
