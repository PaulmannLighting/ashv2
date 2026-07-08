use std::future::Future;
use std::pin::Pin;

pub use self::error::Error;

mod error;

/// Boxed actor future returned by [`crate::start`].
pub type ActorFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
