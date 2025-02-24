//! header line #2 parsing helper

use crate::prelude::{Duration, ParsingError};

use std::io::{BufWriter, Write};
use std::str::FromStr;

pub fn is_header_line2(content: &str) -> bool {
    content.starts_with("##")
}

pub struct Line2 {
    pub week_counter: (u32, f64),
    pub epoch_interval: Duration,
    pub mjd: (u32, f64),
}

impl Line2 {
    pub fn parse(line: &str) -> Result<Self, ParsingError> {
        if line.len() != 60 {
            return Err(ParsingError::MalformedH2);
        }

        let mut mjd = (0_u32, 0.0_f64);
        let mut week_counter = (0_u32, 0.0_f64);

        week_counter.0 = u32::from_str(line[2..7].trim())
            .or(Err(ParsingError::WeekCounter(line[2..7].to_string())))?;

        week_counter.1 = f64::from_str(line[7..23].trim())
            .or(Err(ParsingError::WeekCounter(line[7..23].to_string())))?;

        let dt = f64::from_str(line[24..38].trim())
            .or(Err(ParsingError::EpochInterval(line[24..38].to_string())))?;

        mjd.0 = u32::from_str(line[38..44].trim())
            .or(Err(ParsingError::Mjd(line[38..44].to_string())))?;

        mjd.1 =
            f64::from_str(line[44..].trim()).or(Err(ParsingError::Mjd(line[44..].to_string())))?;

        Ok(Self {
            mjd,
            week_counter,
            epoch_interval: Duration::from_seconds(dt),
        })
    }

    pub fn format<W: Write>(&self, w: &mut BufWriter<W>) -> Result<(), FormattingError> {
        let week_s = self.week_counter.1.integer().round() as u32;
        let week_nanos = (self.week_counter.1.fract() * 1.0E9).round() as u64;

        let dt_s = self.epoch_interval.to_nanoseconds().integer().round() as u16;

        let dt_nanos =
            self.epoch_interval.total_nanoseconds() - (dt_seconds as u64) * 1_000_000_000;

        let mjd_s = self.mjd.1.integer.round() as u32;
        let mut mjd_nanos = (self.mjd.1.fract() * 1.0E9).round() as u64;
        mjd_nanos -= mjd_s as u64 * 1_000_000_000;

        write!(
            w,
            "##    {}          {}.{}        {}.         {:09}    {}.         {:09}   {}.         {:09}",
            self.week_counter.0,
            week_s,
            week_nanos,
            dt_s,
            dt_nanos,
            self.mjd.0,
            mjd_s,
            mjd_nanos,

        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::Line2;
    use std::str::FromStr;

    #[test]
    fn test_line2_parsing() {
        let bad = "##  887      0.00000000   900.00000000 50453 0.0000000000";
        assert!(Line2::parse(bad).is_err());

        for (line, week_counter, week_sow, epoch_interval, mjd, mjd_fract) in [(
            "##  887      0.00000000   900.00000000 50453 0.0000000000000",
            887,
            0.0,
            900.0,
            50453,
            0.0,
        )] {
            let line2 = Line2::parse(&line).unwrap();

            assert_eq!(line2.week_counter.0, week_counter);
            assert_eq!(line2.week_counter.1, week_sow);
            assert_eq!(line2.mjd.0, mjd);
            assert_eq!(line2.mjd.1, mjd_fract);
            assert_eq!(line2.epoch_interval.to_seconds(), epoch_interval);

            line2.format(&mut buf).unwrap();
            assert_eq!(utf8, line2);
        }
    }
}
