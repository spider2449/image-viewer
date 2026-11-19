fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/application.rc");
        println!("cargo:rerun-if-changed=assets/app.ico");
        embed_resource::compile("assets/application.rc", embed_resource::NONE)
            .manifest_required()
            .expect("embed the application icon");
    }
}
