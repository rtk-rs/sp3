//! header line #1 parsing helper

use crate::{
    epoch_decomposition,
    header::{DataType, OrbitType, Version},
    prelude::{FormattingError, ParsingError},
};

use std::io::{BufWriter, Write};
use std::str::FromStr;

pub(crate) fn is_header_line1(content: &str) -> bool {
    content.starts_with('#')
}

pub struct Line1 {
    /// Date & time of publication / release
    pub release_datetime: (i32, u8, u8, u8, u8, u32, u64),
    /// Release revision
    pub version: Version,
    /// Publisher
    pub agency: String,
    /// [DataType]
    pub data_type: DataType,
    /// Coord system description
    pub coord_system: String,
    /// Orbit description
    pub orbit_type: OrbitType,
}

impl Line1 {
    pub fn parse(line: &str) -> Result<Self, ParsingError> {
        if line.len() < 59 {
            return Err(ParsingError::InvalidH1);
        }

        let y = &line[4..8]
            .trim()
            .parse::<i32>()
            .map_err(|_| ParsingError::DateTimeParsing)?;

        let m = &line[4..8]
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DateTimeParsing)?;

        let d = &line[4..8]
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DateTimeParsing)?;

        let hh = &line[4..8]
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DateTimeParsing)?;

        let mm = &line[4..8]
            .trim()
            .parse::<u8>()
            .map_err(|_| ParsingError::DateTimeParsing)?;

        let ss = &line[4..8]
            .trim()
            .parse::<u32>()
            .map_err(|_| ParsingError::DateTimeParsing)?;

        let nanos = &line[4..8]
            .trim()
            .parse::<u64>()
            .map_err(|_| ParsingError::DateTimeParsing)?;

        Ok(Self {
            release_datetime: (y, m, d, hh, mm, ss, nanos),
            version: Version::from_str(&line[1..2])?,
            data_type: DataType::from_str(&line[2..3])?,
            coord_system: line[45..51].trim().to_string(),
            orbit_type: OrbitType::from_str(line[51..55].trim())?,
            agency: line[55..].trim().to_string(),
        })
    }

    pub fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        let (y, m, d, hh, mm, ss, nanos) = self.release_datetime;

        write!(
            w,
            "#{}{}{:04} {} {:2}  {}  {}  {}.{}    {} __u+U {} {}    {}",
            self.version,
            self.data_type,
            y,
            m,
            d,
            hh,
            mm,
            ss,
            nanos,
            0,
            self.coord_system,
            self.orbit_type,
            self.agency
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::Line1;
    use crate::header::OrbitType;
    use crate::header::Version;
    use std::str::FromStr;

    #[test]
    fn test_line1() {
        let bad = "#dP2020  6 24  0  0  0.00000000      97 __u+U IGS14 FIT  ";
        assert!(Line1::parse(bad).is_err());

        for (line, version, coord_system, orbit_type) in [(
            "#dP2020  6 24  0  0  0.00000000      97 __u+U IGS14 FIT  IAC",
            Version::D,
            "IGS14",
            OrbitType::FIT,
        )] {
            let line1 = Line1::parse(&line).unwrap();
            assert_eq!(line1.version, version);
            assert_eq!(line1.coord_system, coord_system);
            assert_eq!(line1.orbit_type, orbit_type);

            // reciprocal
            let formatted = line1.format(&mut buf).unwrap();
            assert_eq!(formatted, line);
        }
    }
}
