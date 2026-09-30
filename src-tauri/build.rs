fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winres::WindowsResource::new()
            .set_icon("icons/icon.ico")
            .set("ProductName", "R5 Battery Estimator V2")
            .set("FileDescription", "R5 Battery Estimator V2")
            .compile()
            .expect("failed to embed the Windows application icon");
    }
}
