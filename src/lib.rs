mod app;
mod chat;
mod tokio;

pub use app::AppPlugin;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
#[tokio::main]
async fn run() {
    App::new().add_plugins(AppPlugin).run();
}
