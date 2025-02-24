//! Velocity entry parsing
use crate::prelude::{ParsingError, SV};
use std::str::FromStr;

pub fn velocity_entry(content: &str) -> bool {
    content.starts_with('V')
}

pub struct VelocityEntry {
    sv: SV,
    velocity: (f64, f64, f64),
    clock: Option<f64>,
}

impl VelocityEntry {
    pub fn to_parts(&self) -> (SV, (f64, f64, f64), Option<f64>) {
        (self.sv, self.velocity, self.clock)
    }

    pub(crate) fn parse(line: &str) -> Result<Self, ParsingError> {
        let mut clock: Option<f64> = None;
        let sv =
            SV::from_str(line[1..4].trim()).or(Err(ParsingError::SV(line[1..4].to_string())))?;
        let x = f64::from_str(line[4..18].trim())
            .or(Err(ParsingError::Coordinates(line[4..18].to_string())))?;
        let y = f64::from_str(line[18..32].trim())
            .or(Err(ParsingError::Coordinates(line[18..32].to_string())))?;
        let z = f64::from_str(line[32..46].trim())
            .or(Err(ParsingError::Coordinates(line[32..46].to_string())))?;

        if !line[45..52].trim().eq("999999.") {
            /*
             * Clock data present
             */
            let clk_data = f64::from_str(line[46..60].trim())
                .or(Err(ParsingError::Clock(line[46..60].to_string())))?;
            clock = Some(clk_data);
        }
        Ok(Self {
            sv,
            velocity: (x, y, z),
            clock,
        })
    }
}
