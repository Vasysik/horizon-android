fn main() {
    // The game's mod hooks are generated as for horizon-client (build.rs there).
    let root =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    horizon_mod_api_gen::generate(&root, &out_dir, horizon_mod_api_gen::Target::Client)
        .expect("generating native mod API");
    // Android tombstones can match a native frame to the exact private symbols.
    // This affects the final ELF only, not code generation or release optimization.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-Wl,--build-id=sha1");
    }
}
