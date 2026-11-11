//! What the backend needs from the application around it: shared state looked
//! up by type, a named event bus the UI listens on, and the Tokio runtime the
//! SSH stack runs on.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use serde::Serialize;

const APP_IDENTIFIER: &str = "com.noverwork.noverterm";

/// The runtime every backend future runs on. The UI has its own executor, so
/// it spawns backend work here and awaits the join handle.
pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to start the Tokio runtime")
    })
}

/// Where the database lives. `NOVERTERM_DATA_DIR` overrides it, for tests and
/// for running a second copy against scratch data.
pub fn data_dir() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("NOVERTERM_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    dirs::data_dir()
        .map(|dir| dir.join(APP_IDENTIFIER))
        .ok_or_else(|| "no application data folder available".to_string())
}

pub struct Event {
    payload: String,
}

impl Event {
    /// The event payload as JSON.
    pub fn payload(&self) -> &str {
        &self.payload
    }
}

type Listener = Arc<dyn Fn(Event) + Send + Sync>;

#[derive(Default)]
struct Inner {
    states: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    listeners: Mutex<HashMap<u32, (String, Listener)>>,
    next_listener: AtomicU32,
}

#[derive(Clone, Default)]
pub struct AppHandle {
    inner: Arc<Inner>,
}

impl AppHandle {
    pub fn manage<T: Send + Sync + 'static>(&self, state: T) {
        self.inner
            .states
            .write()
            .expect("state lock")
            .insert(TypeId::of::<T>(), Arc::new(state));
    }

    /// Panics when `T` was never managed, which is a startup bug.
    pub fn state<T: Send + Sync + 'static>(&self) -> State<'_, T> {
        let state = self
            .inner
            .states
            .read()
            .expect("state lock")
            .get(&TypeId::of::<T>())
            .cloned()
            .unwrap_or_else(|| panic!("state {} is not managed", std::any::type_name::<T>()));
        State {
            value: state
                .downcast()
                .expect("state is stored under its own type"),
            lifetime: PhantomData,
        }
    }

    pub fn emit<P: Serialize>(&self, event: &str, payload: P) -> Result<(), String> {
        let payload = serde_json::to_string(&payload).map_err(|error| error.to_string())?;
        let listeners: Vec<Listener> = self
            .inner
            .listeners
            .lock()
            .map_err(|error| error.to_string())?
            .values()
            .filter(|(name, _)| name == event)
            .map(|(_, listener)| listener.clone())
            .collect();
        // Called outside the lock: a listener may listen or unlisten.
        for listener in listeners {
            listener(Event {
                payload: payload.clone(),
            });
        }
        Ok(())
    }

    pub fn listen(&self, event: &str, listener: impl Fn(Event) + Send + Sync + 'static) -> u32 {
        let id = self.inner.next_listener.fetch_add(1, Ordering::Relaxed);
        self.inner
            .listeners
            .lock()
            .expect("listener lock")
            .insert(id, (event.to_string(), Arc::new(listener)));
        id
    }

    pub fn unlisten(&self, id: u32) {
        self.inner
            .listeners
            .lock()
            .expect("listener lock")
            .remove(&id);
    }
}

pub struct State<'a, T> {
    value: Arc<T>,
    lifetime: PhantomData<&'a ()>,
}

impl<T> State<'_, T> {
    pub fn inner(&self) -> &T {
        &self.value
    }
}

impl<T> Deref for State<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_reach_only_their_listeners_until_unlistened() {
        let app = AppHandle::default();
        app.manage(7_u32);
        assert_eq!(*app.state::<u32>(), 7);

        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let id = app.listen("a", move |event| {
            sink.lock().unwrap().push(event.payload().to_string())
        });
        app.emit("a", 1).unwrap();
        app.emit("b", 2).unwrap();
        app.unlisten(id);
        app.emit("a", 3).unwrap();
        assert_eq!(*seen.lock().unwrap(), ["1"]);
    }
}
