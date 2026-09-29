//! Network providers behind traits. The rest of the app depends on the traits only.

use std::pin::Pin;

pub mod images;
pub mod tts;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
