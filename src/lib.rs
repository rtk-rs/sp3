//! SP3 precise orbit file parser.
#![doc(html_logo_url = "https://raw.githubusercontent.com/rtk-rs/.github/master/logos/logo2.jpg")]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

/*
 * SP3 is part of the rtk-rs framework.
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al.
 * (cf. https://github.com/rtk-rs/sp3/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation: https://github.com/rtk-rs/sp3
 */

extern crate gnss_rs as gnss;

use itertools::Itertools;

#[cfg(feature = "qc")]
extern crate gnss_qc_traits as qc_traits;

use gnss::prelude::{Constellation, SV};
use hifitime::Epoch;
use prelude::ProductionAttributes;
use production::Campaign;

use std::collections::BTreeMap;

#[cfg(feature = "qc")]
#[cfg_attr(docsrs, doc(cfg(feature = "qc")))]
mod qc;

#[cfg(feature = "processing")]
#[cfg_attr(docsrs, doc(cfg(feature = "processing")))]
mod processing;

#[cfg(feature = "nyx-space")]
#[cfg_attr(docsrs, doc(cfg(feature = "nyx-space")))]
mod nyx;

#[cfg(feature = "anise")]
use anise::{
    astro::AzElRange,
    math::Vector6,
    prelude::{Almanac, Frame, Orbit},
};

#[cfg(test)]
mod tests;

mod entry;
mod errors;
mod formatting;
mod header;
mod parsing;
mod position;
mod production;
mod velocity;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use header::Header;
use hifitime::Unit;

use entry::SP3Entry;
use errors::*;

type Vector3D = (f64, f64, f64);

pub mod prelude {
    pub use crate::{
        entry::SP3Entry,
        errors::{Error, FormattingError, ParsingError},
        header::{version::Version, DataType, Header, OrbitType},
        production::{Availability, ProductionAttributes, ReleaseDate, ReleasePeriod},
        SP3Key, SP3,
    };

    #[cfg(feature = "qc")]
    pub use gnss_qc_traits::{Merge, Timeshift};

    #[cfg(feature = "processing")]
    pub use gnss_qc_traits::Split;

    // Pub re-export
    pub use gnss::prelude::{Constellation, SV};
    pub use hifitime::{Duration, Epoch, TimeScale};
}

/// SP3 dataset is a list of [SP3Entry] indexed by [SP3Key].
#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Eq, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SP3Key {
    /// Spacecraft described as [SV]
    pub sv: SV,

    /// Epoch
    pub epoch: Epoch,
}

#[derive(Default, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SP3 {
    /// File [Header]
    pub header: Header,

    /// File header comments, stored as is.
    pub comments: Vec<String>,

    /// [ProductionAttributes] from file names that
    /// follow the standard conventions
    pub prod_attributes: Option<ProductionAttributes>,

    /// File content are [SP3Entry]s sorted per [SP3Key]
    pub data: BTreeMap<SP3Key, SP3Entry>,
}

use crate::prelude::{Availability, DataType, ReleasePeriod};

// Lagrangian interpolator
pub(crate) fn lagrange_interpolation(
    order: usize,
    t: Epoch,
    x: Vec<(Epoch, Vector3D)>,
) -> Option<Vector3D> {
    let x_len = x.len();
    let mut polynomials = Vector3D::default();

    if x_len < order + 1 {
        return None;
    }

    for i in 0..order + 1 {
        let mut l_i = 1.0_f64;
        let (t_i, (x_km_i, y_km_i, z_km_i)) = x[i];

        for j in 0..order + 1 {
            let (t_j, _) = x[j];
            if j != i {
                l_i *= (t - t_j).to_seconds();
                l_i /= (t_i - t_j).to_seconds();
            }
        }

        polynomials.0 += x_km_i * l_i;
        polynomials.1 += y_km_i * l_i;
        polynomials.2 += z_km_i * l_i;
    }

    Some(polynomials)
}

// // 2D Linear interpolation
// pub(crate) fn linear_interpolation(
//     order: usize,
//     t: Epoch,
//     x: Vec<(Epoch, f64)>,
// ) -> Option<f64> {
//
//     let x_len = x.len();
//     if x_len < 2 {
//         return None;
//     }
//
//     let (x_0, y_0) = x[0];
//     let (x_1, y_1) = x[1];
//     let dt = (x_1 - x_0).to_seconds();
//     let mut dy = (x_1 - t).to_seconds() /dt * y_0;
//     dy += (t - x_0).to_seconds() /dt * y_1;
//     Some(dy)
// }

impl SP3 {
    /// Returns [Epoch] of first entry
    pub fn first_epoch(&self) -> Option<Epoch> {
        self.epochs_iter().nth(0)
    }

    /// Returns last [Epoch] to be found in this record.
    pub fn last_epoch(&self) -> Option<Epoch> {
        self.epochs_iter().last()
    }

    /// Returns true if this [SP3] has satellites velocity vector
    pub fn has_satellite_velocity(&self) -> bool {
        self.header.data_type == DataType::Velocity
    }

    /// Returns true if at least one state vector (whatever the constellation)
    /// was predicted
    pub fn has_satellite_positions_prediction(&self) -> bool {
        self.data
            .iter()
            .filter_map(|(k, v)| {
                if v.predicted_orbit {
                    Some((k, v))
                } else {
                    None
                }
            })
            .count()
            > 0
    }

    /// Returns true if at least one clock event was reported for one [SV] (whatever the constellation)
    pub fn has_satellite_clock_event(&self) -> bool {
        self.satellites_epoch_clock_event_iter().count() > 0
    }

    /// Returns true if this [SP3] has satellites clock offset
    pub fn has_satellite_clock_offset(&self) -> bool {
        self.satellites_clock_offset_sec_iter().count() > 0
    }

    /// Returns true if this [SP3] has satellites clock drift
    pub fn has_satellite_clock_drift(&self) -> bool {
        self.satellites_clock_drift_sec_sec_iter().count() > 0
    }

    /// Returns true if at least 1 [SV] (whatever the constellation) is being maneuvered
    /// during this entire time frame
    pub fn has_satellite_maneuver(&self) -> bool {
        self.satellites_epoch_maneuver_iter().count() > 0
    }

    /// Returns true if this [SP3] publication is correct, that is:
    /// - all data points are correctly evenly spaced in time
    /// according to the sampling interval.
    /// You should use this verification method prior any interpolation (post processing).
    pub fn has_steady_sampling(&self) -> bool {
        let dt = self.header.sampling_period;

        let mut t = Epoch::default();
        let mut past_t = Option::<Epoch>::None;

        for now in self.epochs_iter() {
            if now > t {
                // new epoch
                if let Some(past_t) = past_t {
                    if now - past_t != dt {
                        return false;
                    }
                }
                t = now;
            }
            past_t = Some(now);
        }
        true
    }

    /// Propose a file name that would follow the IGS file naming conventions.
    /// This is particularly useful in the context of sP3 data synthesis
    /// and production. It may also be used to generate a file name
    /// that would follow the conventions, while parsed from a file that did not.
    pub fn standardized_filename(&self) -> String {
        let mut batch_id = 0;
        let mut campaign = Campaign::default();
        let mut avail = Availability::default();
        let mut release_period = ReleasePeriod::default();
        let mut agency = self.header.agency[..3].to_string();

        let mut extension = "";

        if let Some(attributes) = &self.prod_attributes {
            batch_id = attributes.batch_id;
            avail = attributes.availability;
            campaign = attributes.campaign;
            release_period = attributes.release_period;
            agency = attributes.agency.clone();
            extension = ".gz";
        }

        let (year, doy) = (
            self.header.release_epoch.year(),
            self.header.release_epoch.day_of_year() as u16,
        );

        let doy_padding = if doy < 100 { "00000" } else { "0000" };

        let sampling_period_mins = (self.header.sampling_period.to_seconds() / 60.0).round() as u16;

        format!(
            "{}{}{}{}_{}{:03}{}_{}_{:02}M_ORB.SP3{}",
            agency,
            batch_id,
            campaign,
            avail,
            year,
            doy as u16,
            doy_padding,
            release_period,
            sampling_period_mins,
            extension,
        )
    }

    /// Returns total number of [Epoch] to be found
    pub fn total_epochs(&self) -> usize {
        self.epochs_iter().count()
    }

    /// Returns [Epoch] [Iterator]
    pub fn epochs_iter(&self) -> impl Iterator<Item = Epoch> + '_ {
        self.data.keys().map(|k| k.epoch).unique()
    }

    /// Returns a unique [Constellation] iterator
    pub fn constellations_iter(&self) -> impl Iterator<Item = Constellation> + '_ {
        self.satellites_iter().map(|sv| sv.constellation).unique()
    }

    /// File comments [Iterator]
    pub fn comments_iter(&self) -> impl Iterator<Item = &String> + '_ {
        self.comments.iter()
    }

    /// Returns a unique [SV] iterator
    pub fn satellites_iter(&self) -> impl Iterator<Item = SV> + '_ {
        self.header.satellites.iter().copied()
    }

    /// [SV] position coordinates [Iterator], in kilometers ECEF, with theoretical 10⁻³m precision.  
    /// All coordinates expressed in fixed body frame. The coordinates system is given by [Header] section.   
    /// The provided [Iterator] contains all coordinates, whether they were fitted or predicted.  
    ///
    /// ## Output
    /// - [Epoch] : sampling epoch
    /// - [SV] : satellite identity
    /// - predicted: true when coordinates are acually predicted
    /// - maneuver: true when satellites is marked under maneuver at this [Epoch]
    /// - [Vector3D] : coordinates
    pub fn satellites_position_km_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, bool, bool, Vector3D)> + '_> {
        Box::new(self.data.iter().filter_map(|(k, v)| {
            Some((k.epoch, k.sv, v.predicted_orbit, v.maneuver, v.position_km))
        }))
    }

    /// [SV] position coordinates [Iterator], in kilometers ECEF, with theoretical 10⁻³m precision.  
    /// All coordinates expressed in fixed body frame. The coordinates system is given by [Header] section.   
    /// The provided [Iterator] contains all coordinates, whether they were fitted or predicted, but
    /// not satellites being maneuvered: this will output a gap during the maneuver duration.
    ///
    /// ## Output
    /// - [Epoch] : sampling epoch
    /// - [SV] : satellite identity
    /// - predicted: true when coordinates are acually predicted
    /// - [Vector3D] : coordinates
    pub fn satellites_stable_position_km_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, bool, Vector3D)> + '_> {
        Box::new(self.satellites_position_km_iter().filter_map(
            |(t, sv, predicted, maneuvered, coords)| {
                if !maneuvered {
                    Some((t, sv, predicted, coords))
                } else {
                    None
                }
            },
        ))
    }

    /// [SV] position coordinates [Iterator], in kilometers ECEF, with theoretical 10⁻³m precision.  
    /// All coordinates expressed in fixed body frame. The coordinates system is given by [Header] section.   
    /// The provided [Iterator] contains only fitted coordinates (not predicted), and not satellites being maneuvered.
    /// This will output a data gap during the maneuver duration.
    ///
    /// ## Output
    /// - [Epoch] : sampling epoch
    /// - [SV] : satellite identity
    /// - [Vector3D] : coordinates
    pub fn satellites_stable_fitted_position_km_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, Vector3D)> + '_> {
        Box::new(self.satellites_stable_position_km_iter().filter_map(
            |(t, sv, predicted, coords)| {
                if !predicted {
                    Some((t, sv, coords))
                } else {
                    None
                }
            },
        ))
    }

    /// [SV] position coordinates [Iterator], in kilometers ECEF, with theoretical 10⁻³m precision.  
    /// All coordinates expressed in fixed body frame. The coordinates system is given by [Header] section.   
    /// The provided [Iterator] contains only predicted coordinates (not fitted), and not satellites being maneuvered.
    /// This will output a data gap during the maneuver duration.
    ///
    /// ## Output
    /// - [Epoch] : sampling epoch
    /// - [SV] : satellite identity
    /// - [Vector3D] : coordinates
    pub fn satellites_stable_predicted_position_km_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, Vector3D)> + '_> {
        Box::new(self.satellites_stable_position_km_iter().filter_map(
            |(t, sv, predicted, coords)| {
                if predicted {
                    Some((t, sv, coords))
                } else {
                    None
                }
            },
        ))
    }

    /// [SV] [Orbit]al state [Iterator] with theoretical 10⁻³m precision.
    /// For this to be correct:
    /// - [Frame] must be ECEF
    /// - [Frame] should match the coordinates system described in [Header]
    /// NB: all satellites being maneuvered are sorted out, which makes this method
    /// compatible with navigation.
    #[cfg(feature = "anise")]
    #[cfg_attr(docsrs, doc(cfg(feature = "anise")))]
    pub fn satellites_orbit_iter(
        &self,
        frame_cef: Frame,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, Orbit)> + '_> {
        Box::new(self.data.iter().filter_map(move |(k, v)| {
            if !v.maneuver {
                let (x_km, y_km, z_km) = v.position_km;
                let (vx_km_s, vy_km_s, vz_km_s) = match v.velocity_km_s {
                    Some((vx_km_s, vy_km_s, vz_km_s)) => (vx_km_s, vy_km_s, vz_km_s),
                    None => (0.0, 0.0, 0.0),
                };

                let pos_vel = Vector6::new(x_km, y_km, z_km, vx_km_s, vy_km_s, vz_km_s);
                let orbit = Orbit::from_cartesian_pos_vel(pos_vel, k.epoch, frame_cef);
                Some((k.epoch, k.sv, orbit))
            } else {
                None
            }
        }))
    }

    /// [SV] (elevation, azimuth, range) attitude vector [Iterator], as [AzElRange].
    #[cfg(feature = "anise")]
    #[cfg_attr(docsrs, doc(cfg(feature = "anise")))]
    pub fn satellites_elevation_azimuth_iter(
        &self,
        almanac: Almanac,
        frame_cef: Frame,
        rx_orbit: Orbit,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, AzElRange)> + '_> {
        Box::new(
            self.satellites_orbit_iter(frame_cef)
                .filter_map(move |(sv, t, tx_orbit)| {
                    if let Ok(elazrng) =
                        almanac.azimuth_elevation_range_sez(rx_orbit, tx_orbit, None, None)
                    {
                        Some((sv, t, elazrng))
                    } else {
                        None
                    }
                }),
        )
    }

    /// Returns ([Epoch], [SV]) [Iterator] where satellite maneuver is being reported
    pub fn satellites_epoch_maneuver_iter(&self) -> Box<dyn Iterator<Item = (Epoch, SV)> + '_> {
        Box::new(self.data.iter().filter_map(|(k, v)| {
            if v.maneuver {
                Some((k.epoch, k.sv))
            } else {
                None
            }
        }))
    }

    /// Returns ([Epoch], [SV]) [Iterator] where satellite clock
    /// event flag was asserted.
    pub fn satellites_epoch_clock_event_iter(&self) -> Box<dyn Iterator<Item = (Epoch, SV)> + '_> {
        Box::new(self.data.iter().filter_map(|(k, v)| {
            if v.clock_event {
                Some((k.epoch, k.sv))
            } else {
                None
            }
        }))
    }

    /// Returns an [Iterator] over [SV] velocity vector, in km.s⁻¹
    /// and 0.1 10⁻⁷m precision, for all satellites in correct Orbit (not being maneuvered).
    pub fn satellites_velocity_km_s_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, Vector3D)> + '_> {
        Box::new(self.data.iter().filter_map(|(k, v)| {
            if !v.maneuver {
                let velocity_km_s = v.velocity_km_s?;
                Some((k.epoch, k.sv, velocity_km_s))
            } else {
                None
            }
        }))
    }

    /// Forms an absolute position (in km ECEF) and instant. velocity vector (in km.s⁻¹) [Iterator].
    pub fn satellites_pos_vel_km_iter(
        &self,
    ) -> Box<dyn Iterator<Item = (Epoch, SV, Vector3D, Vector3D)> + '_> {
        Box::new(self.data.iter().filter_map(|(k, v)| {
            if !v.maneuver {
                let position_km = v.position_km;
                let velocity_km = v.velocity_km_s?;
                Some((k.epoch, k.sv, position_km, velocity_km))
            } else {
                None
            }
        }))
    }

    /// [SV] clock offset in seconds (with 10⁻¹² theoretical precision) [Iterator].
    pub fn satellites_clock_offset_sec_iter(&self) -> impl Iterator<Item = (Epoch, SV, f64)> + '_ {
        self.data.iter().filter_map(|(k, v)| {
            let clock = v.clock_us? * 1.0E-6;
            Some((k.epoch, k.sv, clock))
        })
    }

    /// [SV] clock offset in s.s⁻¹ (with 10⁻¹⁶ theoretical precision) [Iterator].
    pub fn satellites_clock_drift_sec_sec_iter(
        &self,
    ) -> impl Iterator<Item = (Epoch, SV, f64)> + '_ {
        self.data.iter().filter_map(|(k, v)| {
            let rate = v.clock_drift_ns? * 1.0E-9;
            Some((k.epoch, k.sv, rate))
        })
    }

    /// Designs an evenly spaced (in time) grouping of (x_km, y_km, z_km) coordinates
    /// for you to apply your own interpolation method (as a function pointer).
    /// NB:
    /// - This only works on correct SP3 publications with steady sample rate.
    /// - There is no internal verification here, you should verify the correctness
    ///  of the SP3 publication with [Self::has_steady_sampling] prior running this.
    /// ## Input
    /// - sv: selected [SV]
    /// - t: Interpolation [Epoch]
    /// - order: Interpolation order. Only odd interpolation order is supported.
    /// This method will panic on even interpolation order.
    /// - interp: function pointer for this order and epoch, and the time frame.
    /// The time frame being [(N +1)/2 * τ;  (N +1)/2 * τ].
    /// A 7th order will create an 8 data point window.
    /// ## Output
    /// - Your function pointer should return Option<(f64, f64, f64)>,
    /// and is summoned with `order`, `t` and the time frame.
    /// - This method will return None if `t` is either
    /// to early or too late with respect to interpolation order.
    /// That means, we only generate perfectly centered time frames,
    /// to minimize interpolation error.
    pub fn satellite_position_interpolate(
        &self,
        sv: SV,
        t: Epoch,
        order: usize,
        interp: fn(usize, Epoch, Vec<(Epoch, Vector3D)>) -> Option<Vector3D>,
    ) -> Option<Vector3D> {
        let odd_order = order % 2 > 0;
        if !odd_order {
            panic!("even interpolation order is not supported");
        }

        // delta interval for which we consider Epoch equality
        let smallest_dt = 2.0 * Unit::Nanosecond;

        let target_len = order + 1;
        let target_len_2 = target_len / 2;
        let target_len_2_1 = target_len_2 - 1;

        let mut past_t = Epoch::default();

        let mut t_x = Option::<Epoch>::None;
        let mut tx_perfect_match = false;
        let (mut w0_len, mut w1_len) = (0, 0);

        let mut window = Vec::<(Epoch, Vector3D)>::with_capacity(target_len);

        for (index_i, (t_i, sv_i, _, (x_i, y_i, z_i))) in
            self.satellites_stable_position_km_iter().enumerate()
        {
            if sv_i != sv {
                past_t = t_i;
                continue;
            }

            // always push while maintaining correct size
            window.push((t_i, (x_i, y_i, z_i)));

            let win_len = window.len();
            if win_len > target_len {
                window.remove(0);
            }

            if t_x.is_none() {
                if past_t < t && t_i >= t {
                    // found t_x
                    w0_len = index_i;
                    t_x = Some(t_i);

                    if (t_i - t).abs() < smallest_dt {
                        tx_perfect_match = true;
                    }
                }
            } else {
                // stop when window has been gathered
                if index_i == w0_len + target_len_2 - 1 {
                    w1_len = target_len_2;
                    break;
                }
            }

            past_t = t_i;
        }

        t_x?;

        // central point must not be too early
        if w0_len < target_len_2 {
            return None;
        }

        // println!("t_x={} [{} ; {}]", t_x, w0_len, w1_len); // DEBUG

        // window must be correctly centered on central point
        if tx_perfect_match {
            if w1_len < target_len_2_1 {
                return None;
            }
        } else if w1_len < target_len_2 {
            return None;
        }

        interp(order, t, window)
    }

    /// Applies the Lagrangian interpolation method
    /// at desired [Epoch] `t` using desired interpoation order,
    /// as per <https://www.math.univ-paris13.fr/~japhet/L2/2020-2021/Interpolation.pdf>
    /// NB:
    /// - this will panic on even interpolation orders
    /// - this will not interpolate (returns None) if [Epoch]
    /// is either too early or too late with respect to
    /// interpolation order.
    pub fn satellite_position_lagrangian_interpolation(
        &self,
        sv: SV,
        t: Epoch,
        order: usize,
    ) -> Option<Vector3D> {
        self.satellite_position_interpolate(sv, t, order, lagrange_interpolation)
    }

    /// Applies 9th order Lagrangian interpolation method, which is compatible with high precision geodesy.
    /// See [Self::satellite_position_lagrangian_interpolation].
    pub fn satellite_position_lagrangian_9_interpolation(
        &self,
        sv: SV,
        t: Epoch,
    ) -> Option<Vector3D> {
        self.satellite_position_lagrangian_interpolation(sv, t, 9)
    }

    /// Applies 11th order Lagrangian interpolation method, which is compatible with high precision geodesy.
    /// See [Self::satellite_position_lagrangian_interpolation].
    pub fn satellite_position_lagrangian_11_interpolation(
        &self,
        sv: SV,
        t: Epoch,
    ) -> Option<Vector3D> {
        self.satellite_position_lagrangian_interpolation(sv, t, 11)
    }

    /// Applies 17th order Lagrangian interpolation method, which is compatible with high precision geodesy.
    /// See [Self::satellite_position_lagrangian_interpolation].
    pub fn satellite_position_lagrangian_17_interpolation(
        &self,
        sv: SV,
        t: Epoch,
    ) -> Option<Vector3D> {
        self.satellite_position_lagrangian_interpolation(sv, t, 17)
    }
}
