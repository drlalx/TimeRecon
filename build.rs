#[cfg(windows)]
fn embed_windows_icon() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icons/app.ico");
        let _ = resource.compile();
    }
}

#[cfg(not(windows))]
fn embed_windows_icon() {}

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/app.ico");
    embed_windows_icon();
}
