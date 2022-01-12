use std::fs::File;

use anyhow::*;
use simplelog::*;

pub fn initialize_logger(log_level: LevelFilter) -> Result<()> {
    CombinedLogger::init(vec![
        TermLogger::new(log_level, Config::default(), TerminalMode::Mixed),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create("engine_run.log")?,
        ),
    ])?;
    Ok(())
}
