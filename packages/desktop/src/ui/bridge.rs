//! Crossing between the UI executor and the Tokio runtime the backend runs on.

use std::future::Future;

use gpui_kit::{App, Global};

use crate::app::AppHandle;

/// The backend handle, reachable from any view.
pub struct Backend(pub AppHandle);

impl Global for Backend {}

pub fn app(cx: &App) -> AppHandle {
    cx.global::<Backend>().0.clone()
}

/// Runs a backend future on Tokio; the returned future can be awaited from a
/// UI task. Backend commands use Tokio I/O and timers, which the UI executor
/// cannot drive.
pub fn backend<T: Send + 'static>(
    future: impl Future<Output = T> + Send + 'static,
) -> impl Future<Output = T> {
    let task = crate::app::runtime().spawn(future);
    async move { task.await.expect("backend task panicked") }
}
