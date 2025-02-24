//! File descriptor
use crate::prelude::{Constellation, ParsingError, TimeScale};

use std::str::FromStr;

pub fn is_file_descriptor(content: &str) -> bool {
    content.starts_with("%c")
}

pub enum FileDescriptor {
    Line1(FileDescriptorH1),
    Continuation,
}

impl FileDescriptor {
    pub fn parse(line: &str) -> Result<Self, ParsingError> {
        if let Ok(h1) = FileDescriptorH1::parse(line) {
            Ok(Self::Line1(h1))
        } else {
            Ok(Self::Continuation) // TODO
        }
    }
}

pub struct FileDescriptorH1 {
    pub timescale: TimeScale,
    pub constellation: Constellation,
}

impl FileDescriptorH1 {
    pub fn parse(line: &str) -> Result<Self, ParsingError> {
        if line.len() < 60 {
            return Err(ParsingError::InvalidFileDescriptorH1)?;
        }

        let constellation = Constellation::from_str(line[3..5].trim())?;
        let timescale = TimeScale::from_str(line[9..12].trim())?;

        Ok(Self {
            constellation,
            timescale,
        })
    }
}
