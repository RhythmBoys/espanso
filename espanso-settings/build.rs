fn main() {
    println!("cargo:rerun-if-changed=ui/settings.slint");
    if std::env::var_os("CARGO_FEATURE_UI").is_some() {
        slint_build::compile("ui/settings.slint").expect("unable to compile Settings UI");
    }
}
