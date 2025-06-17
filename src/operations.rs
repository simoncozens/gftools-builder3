mod glyphs2ufo;

use std::{process::Output, sync::Arc};

use crate::error::ApplicationError;

pub trait Operation {
    fn execute(&self) -> Result<Output, ApplicationError>;
}
