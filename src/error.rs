use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("Non supported SP3 revision")]
    NonSupportedRevision,
    #[error("Unknown SP3 orbit type")]
    UnknownOrbitType,
    #[error("Unknown SP3 data type")]
    UnknownDataType,
    #[error("missing header line #1")]
    MissingH1,
    #[error("missing header line #2")]
    MissingH2,
    #[error("invalid %c line")]
    InvalidFileDescriptorH1,
    #[error("malformed header line #2")]
    MalformedH2,
    #[error("failed to parse date/time")]
    DatetTimeParsing,
    #[error("failed to parse hifitime::Epoch")]
    Epoch,
    #[error("failed to parse (x, y, or z) coordinates from \"{0}\"")]
    Coordinates(String),
    #[error("failed to parse clock data from \"{0}\"")]
    Clock(String),
}

#[derive(Debug)]
pub enum FormattingError {}
