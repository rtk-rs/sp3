use crate::{
    header::{
        descriptor::{is_file_descriptor, FileDescription},
        line1::Line1,
        line2::Line2,
    },
    prelude::{Header, ParsingError},
};

use std::io::{BufRead, BufReader, Read};

impl Header {
    pub(crate) fn parse<R: Read>(r: &mut BufReader<R>) -> Result<Self, ParsingError> {
        let mut lines = r.lines();
        let mut header = Self::default();

        let h1 = lines.next().ok_or(ParsingError::MissingH1)?;

        let h1 = Line1::parse(h1)?;

        header.version = h1.version;
        header.data_type = h1.data_type;
        header.coord_system = h1.coord_system;
        header.orbit_type = h1.orbit_type;
        header.agency = h1.agency;

        let h2 = lines.next().ok_or(ParsingError::MissingH2)?;

        let h2 = Line1::parse(h1)?;

        header.week_counter = h2.week_counter;
        header.week_sow = h2.week_sow;
        header.epoch_interval = h2.epoch_interval;
        header.mjd = h2.mjd_int as f64;
        header.mjd += h2.mjd_fract;

        for line in r.lines() {
            let line = line.unwrap().trim();

            if sp3_comment(line) {
                if line.len() > 4 {
                    header.comments.push(line[3..].to_string());
                }
            }

            if is_file_descriptor(line) {
                let descriptor = FileDescription::parse(line)?;

                match descriptor {
                    FileDescription::Line1(h1) => {
                        header.timescale = h1.timescale;
                        header.constellation = h1.constellation;
                    },
                    FileDescription::Continuation => {},
                }
            }
        }

        Ok(header)
    }
}
