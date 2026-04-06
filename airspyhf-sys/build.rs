use std::env;
use std::path::PathBuf;

fn main() {
    // Link the AirspyHF library
    println!("cargo:rustc-link-lib=airspyhf");
    println!("cargo:rustc-link-search=native=/usr/local/lib");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .allowlist_function("airspyhf_.*")
        .allowlist_type("airspyhf_.*")
        .allowlist_var("AIRSPYHF_.*")
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
