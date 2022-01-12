const SHADER_DIR: &str = "compute_shaders";

fn main() {
    println!("cargo:rerun-if-changed={}", SHADER_DIR);
    if std::env::var("MAC_OS_BUILD").is_ok() {
        println!("cargo:rustc-link-lib=framework=ColorSync");
    }
}
