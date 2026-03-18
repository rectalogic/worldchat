use std::panic;

use bevy::{ecs::resource::Resource, prelude::Deref};
use tokio::{runtime::Handle, task::JoinHandle};

#[derive(Resource, Deref)]
pub struct TokioRuntime(Handle);

pub struct Task<T>(Option<JoinHandle<T>>);

impl TokioRuntime {
    pub fn new() -> Self {
        Self(Handle::current())
    }
}

impl<T> Task<T> {
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self(Some(handle))
    }

    pub fn spawn<F>(runtime: &TokioRuntime, future: F) -> Self
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        Self::new(runtime.spawn(future))
    }

    pub fn is_finished(&self) -> bool {
        if let Some(ref join_handle) = self.0 {
            join_handle.is_finished()
        } else {
            true
        }
    }

    pub fn result(&mut self, runtime: &TokioRuntime) -> Option<T> {
        if self.is_finished()
            && let Some(join_handle) = self.0.take()
        {
            match runtime.block_on(join_handle) {
                Ok(result) => Some(result),
                Err(e) => {
                    if let Ok(reason) = e.try_into_panic() {
                        panic::resume_unwind(reason);
                    }
                    None
                }
            }
        } else {
            None
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}
