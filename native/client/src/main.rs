#[cfg(not(target_arch = "wasm32"))]
#[path = "main_desktop.rs"]
mod desktop;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    desktop::run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
