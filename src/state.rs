/// State must be implemented for any type being used as the App's state
///
/// Before processing a request the State is "instantiated" which should
/// create a new State object, copying over any global data, and creating
/// default values for request local data
pub trait State: Send + Sync + 'static {
    /// Instantiate the State object, creating a new one to be used for a single request
    fn instantiate(&self) -> Self;
}

/// implement state for all types already meeting the constraints
impl State for ()
{
    fn instantiate(&self) -> Self {
        ()
    }
}
