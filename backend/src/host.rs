//! What the backend needs from the application around it (the Electron shell): a handle that
//! shares state between commands, pushes events to the interface, and a few async helpers.
//!
//! The backend is a separate process that talks to Electron in JSON lines over stdin / stdout:
//! see `rpc.rs`. Events leave through [`AppHandle::emit`], one line each.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::io::Write;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::json;

/// Shared handle on the running backend. Cheap to clone, usable from any thread.
#[derive(Clone)]
pub struct AppHandle(Arc<Inner>);

struct Inner {
    states: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    data_dir: PathBuf,
    out: Mutex<std::io::Stdout>,
}

/// A value registered with [`AppHandle::manage`], borrowed for the length of a command.
pub struct State<'a, T: ?Sized>(Arc<T>, PhantomData<&'a ()>);

impl<T: ?Sized> Deref for State<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl AppHandle {
    pub fn new(data_dir: PathBuf) -> Self {
        Self(Arc::new(Inner {
            states: Mutex::default(),
            data_dir,
            out: Mutex::new(std::io::stdout()),
        }))
    }

    /// Where Lutra keeps its data (`~/.local/share/dev.lutra.app`).
    pub fn data_dir(&self) -> PathBuf {
        self.0.data_dir.clone()
    }

    /// Registers a value that commands can then ask for with [`AppHandle::state`].
    pub fn manage<T: Send + Sync + 'static>(&self, value: T) {
        if let Ok(mut states) = self.0.states.lock() {
            states.insert(TypeId::of::<T>(), Arc::new(value));
        }
    }

    /// A managed value. Panics when it was never registered: that is a bug, not a runtime error.
    pub fn state<T: Send + Sync + 'static>(&self) -> State<'_, T> {
        let value = self
            .0
            .states
            .lock()
            .expect("state registry")
            .get(&TypeId::of::<T>())
            .cloned()
            .unwrap_or_else(|| panic!("{} is not managed", std::any::type_name::<T>()));
        let value = value
            .downcast::<T>()
            .unwrap_or_else(|_| unreachable!("the registry is keyed by type"));
        State(value, PhantomData)
    }

    /// Writes one line to the host. Lines are never interleaved.
    pub fn write_line(&self, line: &str) {
        if let Ok(mut out) = self.0.out.lock() {
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        }
    }

    /// Pushes an event to the interface (or, for names starting with `host:`, to the shell itself).
    pub fn emit<S: Serialize>(&self, event: &str, payload: S) -> Result<(), String> {
        let line = serde_json::to_string(&json!({ "event": event, "payload": payload }))
            .map_err(|e| e.to_string())?;
        self.write_line(&line);
        Ok(())
    }
}

/// The few async helpers the commands use. Every command runs on its own thread, so blocking
/// work is simply done in place and the futures are ready as soon as they are created.
pub mod async_runtime {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    pub fn spawn_blocking<F, R>(work: F) -> std::future::Ready<Result<R, String>>
    where
        F: FnOnce() -> R,
    {
        std::future::ready(Ok(work()))
    }

    /// Drives a command's future to the end.
    pub fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
            std::thread::yield_now();
        }
    }
}
