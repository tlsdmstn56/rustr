// Example custom build script.
#[allow(dead_code)]
fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rustc-link-lib=dylib=mylib");
    // Use the `cc` crate to build a C file and statically link it.
    // cc::Build::new()
    //    .file("src/hello.c")
    //    .compile("hello");
}
