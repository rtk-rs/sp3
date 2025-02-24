use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
    str::FromStr,
};

#[cfg(feature = "flate2")]
use flate2::read::GzDecoder;

use crate::{
    position::{position_entry, PositionEntry},
    prelude::{Epoch, Error, Header, ParsingError, SP3Entry, SP3Key, TimeScale, SP3, SV},
    velocity::{velocity_entry, VelocityEntry},
};

fn end_of_file(content: &str) -> bool {
    content.eq("EOF")
}

fn new_epoch(content: &str) -> bool {
    content.starts_with("*  ")
}

/// Parses [Epoch] from standard SP3 format
fn parse_epoch(content: &str, timescale: TimeScale) -> Result<Epoch, ParsingError> {
    let y = u32::from_str(content[0..4].trim())
        .or(Err(ParsingError::EpochYear(content[0..4].to_string())))?;

    let m = u32::from_str(content[4..7].trim())
        .or(Err(ParsingError::EpochMonth(content[4..7].to_string())))?;

    let d = u32::from_str(content[7..10].trim())
        .or(Err(ParsingError::EpochDay(content[7..10].to_string())))?;

    let hh = u32::from_str(content[10..13].trim())
        .or(Err(ParsingError::EpochHours(content[10..13].to_string())))?;

    let mm = u32::from_str(content[13..16].trim())
        .or(Err(ParsingError::EpochMinutes(content[13..16].to_string())))?;

    let ss = u32::from_str(content[16..19].trim())
        .or(Err(ParsingError::EpochSeconds(content[16..19].to_string())))?;

    let _ss_fract = f64::from_str(content[20..27].trim()).or(Err(
        ParsingError::EpochMilliSeconds(content[20..27].to_string()),
    ))?;

    Epoch::from_str(&format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02} {}",
        y, m, d, hh, mm, ss, timescale,
    ))
    .or(Err(ParsingError::Epoch))
}

impl SP3 {
    /// Parse [SP3] data from local file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ParsingError> {
        let fd = File::open(path).unwrap_or_else(|e| panic!("File open error: {}", e));

        let mut reader = BufReader::new(fd);

        Self::parse(&mut reader)
    }

    #[cfg(feature = "flate2")]
    #[cfg_attr(docsrs, doc(cfg(feature = "flate2")))]
    /// Parse [SP3] data from gzip encoded local file.
    pub fn from_gzip_file(path: impl AsRef<Path>) -> Result<Self, ParsingError> {
        let fd = File::open(path).unwrap_or_else(|e| panic!("File open error: {}", e));

        let fd = GzDecoder::new(fd);
        let mut reader = BufReader::new(fd);

        Self::parse(&mut reader)
    }

    /// Parse [SP3] data from [Read]able I/O.
    pub fn parse<R: Read>(reader: &mut BufReader<R>) -> Result<Self, ParsingError> {
        let mut pc_count = 0_u8;
        let mut header = Header::default();
        let mut timescale = TimeScale::default();

        let mut vehicles: Vec<SV> = Vec::new();
        let mut data = BTreeMap::<SP3Key, SP3Entry>::new();

        let mut epoch = Epoch::default();

        let header = Header::parse(reader)?;

        for line in reader.lines() {
            let line = line.unwrap();
            let line = line.trim();

            if end_of_file(line) {
                break;
            }

            if new_epoch(line) {
                epoch = parse_epoch(&line[3..], timescale)?;
            }

            if position_entry(line) {
                if line.len() < 60 {
                    // tolerates malformed position vectors
                    continue;
                }

                let entry = PositionEntry::parse(line)?;

                //TODO : move this into %c config frame
                if !vehicles.contains(&entry.sv) {
                    vehicles.push(entry.sv);
                }

                // verify entry validity
                if entry.x_km != 0.0_f64 && entry.y_km != 0.0_f64 && entry.z_km != 0.0_f64 {
                    let key = SP3Key {
                        epoch,
                        sv: entry.sv,
                    };

                    if let Some(e) = data.get_mut(&key) {
                        e.position_km = (entry.x_km, entry.y_km, entry.z_km);
                        e.maneuver = entry.maneuver;
                        e.orbit_prediction = entry.orbit_prediction;
                    } else {
                        if let Some(clk_us) = entry.clock_us {
                            let value = if entry.orbit_prediction {
                                SP3Entry::from_predicted_position_km((
                                    entry.x_km, entry.y_km, entry.z_km,
                                ))
                            } else {
                                SP3Entry::from_position_km((entry.x_km, entry.y_km, entry.z_km))
                            };

                            let mut value = if entry.clock_prediction {
                                value.with_predicted_clock_offset_us(clk_us)
                            } else {
                                value.with_clock_offset_us(clk_us)
                            };

                            value.maneuver = entry.maneuver;
                            value.clock_event = entry.clock_event;

                            data.insert(key, value);
                        } else {
                            let mut value = if entry.orbit_prediction {
                                SP3Entry::from_predicted_position_km((
                                    entry.x_km, entry.y_km, entry.z_km,
                                ))
                            } else {
                                SP3Entry::from_position_km((entry.x_km, entry.y_km, entry.z_km))
                            };

                            value.maneuver = entry.maneuver;
                            value.clock_event = entry.clock_event;

                            data.insert(key, value);
                        }
                    }
                }
            }

            if velocity_entry(line) {
                if line.len() < 60 {
                    // tolerates malformed velocity vectors
                    continue;
                }

                let entry = VelocityEntry::parse(line)?;
                let (sv, (vel_x_dm_s, vel_y_dm_s, vel_z_dm_s), clk_sub_ns) = entry.to_parts();

                let (vel_x_km_s, vel_y_km_s, vel_z_km_s) = (
                    vel_y_dm_s * 1.0E-4,
                    vel_y_dm_s * 1.0E-4,
                    vel_z_dm_s * 1.0E-4,
                );

                //TODO : move this into %c config frame
                if !vehicles.contains(&sv) {
                    vehicles.push(sv);
                }

                // verify entry validity
                if vel_x_dm_s != 0.0_f64 && vel_y_dm_s != 0.0_f64 && vel_z_dm_s != 0.0_f64 {
                    let key = SP3Key { epoch, sv };
                    if let Some(e) = data.get_mut(&key) {
                        *e = e.with_velocity_km_s((vel_x_km_s, vel_y_km_s, vel_z_km_s));

                        if let Some(clk_sub_ns) = clk_sub_ns {
                            *e = e.with_clock_drift_ns(clk_sub_ns * 0.1);
                        }
                    } else {
                        // Entry does not exist (velocity prior position)
                        // Should not exist, but we tolerate
                        if let Some(clk_sub_ns) = clk_sub_ns {
                            data.insert(
                                key,
                                SP3Entry::from_position_km((0.0, 0.0, 0.0))
                                    .with_velocity_km_s((vel_x_km_s, vel_y_km_s, vel_z_km_s))
                                    .with_clock_drift_ns(clk_sub_ns * 0.1),
                            );
                        } else {
                            data.insert(
                                key,
                                SP3Entry::from_position_km((0.0, 0.0, 0.0))
                                    .with_velocity_km_s((vel_x_km_s, vel_y_km_s, vel_z_km_s)),
                            );
                        }
                    }
                }
            }
        }
        Ok(Self { header, data })
    }
}
