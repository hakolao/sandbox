use std::io;

#[cfg(windows)]
use winres::WindowsResource;

const SHADER_DIR: &str = "simulation_shaders";

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", SHADER_DIR);
    #[cfg(windows)]
    {
        println!(
            "Adding windows resource image {}",
            "assets/object_images/wizard.png"
        );
        WindowsResource::new()
            .set_icon("assets/object_images/wizard.png")
            .compile()?;
    }
    if std::env::var("MAC_OS_BUILD").is_ok() {
        println!("cargo:rustc-link-lib=framework=ColorSync");
    }

    Ok(())
}
