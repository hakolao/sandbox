use std::io;

#[cfg(windows)]
use winres::WindowsResource;

const SHADER_DIR: &str = "simulation_shaders";

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", SHADER_DIR);
    #[cfg(windows)]
    {
        WindowsResource::new()
            .set_icon("../assets/object_images/wizard.png")
            .compile()?;
    }
    Ok(())
}
