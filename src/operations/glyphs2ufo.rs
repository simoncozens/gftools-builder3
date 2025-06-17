use std::process::Output;

use crate::{error::ApplicationError, operations::Operation};

struct Glyphs2UFO {
    pub source: String,
    pub outputs: Vec<String>,
}

impl Operation for Glyphs2UFO {
    fn execute(&self) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ufo --instance-dir instance_ufo -g {}",
            self.source
        );
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))
    }
}
