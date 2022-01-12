const SHADER_DIR: &str = "shaders";

fn main() {
    println!("cargo:rerun-if-changed={}", SHADER_DIR);
}
