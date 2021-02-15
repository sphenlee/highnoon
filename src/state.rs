
/// State must be implemented for any type being used as the App's state
///
/// State is cloned for each request - if you need any state to be shared wrap it with
/// an Arc
pub trait State: Send + Sync + 'static {
    fn instantiate(&self) -> Self;
}

/// implement state for all types already meeting the constraints
impl State for ()
{
    fn instantiate(&self) -> Self {
        ()
    }
}
