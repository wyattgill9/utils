use bindgen;

fn main() {
    println!("cargo:rustc-link-lib=static=lib");
    println!("cargo:rustc-link-search=native=path/to/library");

    let bindings = bindgen::Builder::default()
        .header("./clibs/example.h") 
        .parse_callbacks(Box::new(CargoCallbacks::new())) 
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file("src/bindings.rs")
        .expect("Couldn't write bindings!");
}
