fn main() {
    println!("cargo:rerun-if-changed=ui/settings.slint");
    if std::env::var_os("CARGO_FEATURE_UI").is_some() {
        // The window paints its own light surfaces (#f5f7fb, white cards), but
        // std-widgets follow the system color scheme. Under a dark desktop the
        // fluent palette turns label text near-white and gives controls a
        // translucent white fill, which over a white card makes `Edit` and
        // `Delete` disappear entirely. Pin the light variant so the whole
        // window stays consistent.
        let config = slint_build::CompilerConfiguration::new().with_style("fluent-light".into());
        slint_build::compile_with_config("ui/settings.slint", config)
            .expect("unable to compile Settings UI");
    }
}
