fn main() {
    #[cfg(target_os = "windows")]
    embed_windows_resources();
}

#[cfg(target_os = "windows")]
fn embed_windows_resources() {
    let mut resources = winresource::WindowsResource::new();
    resources.set_icon("packaging/icons/requiescat.ico");
    resources.set("ProductName", "Requiescat");
    resources.set("FileDescription", "Requiescat");
    resources.set("InternalName", "Requiescat");
    resources.set("LegalCopyright", "Copyright (c) Requiescat");

    resources
        .compile()
        .expect("failed to compile Windows resources");
}
