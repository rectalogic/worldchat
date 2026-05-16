mod app;
mod irc;
mod world;
pub use app::AppPlugin;

#[cfg(target_arch = "wasm32")]
mod wasm;
