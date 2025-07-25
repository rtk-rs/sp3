use crate::prelude::{Epoch, SP3, SV};

use anise::{
    astro::AzElRange,
    math::Vector6,
    prelude::{Almanac, Frame, Orbit},
};

#[cfg(feature = "anise")]
#[cfg_attr(docsrs, doc(cfg(feature = "anise")))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SatelliteOrbitalState {
    /// [Epoch]
    pub epoch: Epoch,

    /// Satellite as [SV]
    pub satellite: SV,

    /// [Orbit]al state
    pub orbit: Orbit,

    /// True when this satellite is undergoing a maneuver
    pub maneuver: bool,
}

#[cfg(feature = "anise")]
#[cfg_attr(docsrs, doc(cfg(feature = "anise")))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SatelliteOrbitalAttitude {
    /// Satellite as [SV]
    pub satellite: SV,

    /// True when this satellite is undergoing a maneuver
    pub maneuver: bool,

    /// Orbital attitude as [AzElRange]
    pub azelrange: AzElRange,
}

impl SP3 {
    /// Form a [SatelliteOrbitalState]s [Iterator].
    ///
    /// ## Inputs
    /// - frame: [Frame] definition that must be an ECEF for this to be correct.
    #[cfg(feature = "anise")]
    #[cfg_attr(docsrs, doc(cfg(feature = "anise")))]
    pub fn satellites_orbit_iter(
        &self,
        frame: Frame,
    ) -> Box<dyn Iterator<Item = SatelliteOrbitalState> + '_> {
        Box::new(self.data.iter().filter_map(move |(k, v)| {
            let (x_km, y_km, z_km) = v.position_km;

            let (vx_km_s, vy_km_s, vz_km_s) = match v.velocity_km_s {
                Some((vx_km_s, vy_km_s, vz_km_s)) => (vx_km_s, vy_km_s, vz_km_s),
                None => (0.0, 0.0, 0.0),
            };

            let pos_vel = Vector6::new(x_km, y_km, z_km, vx_km_s, vy_km_s, vz_km_s);
            let orbit = Orbit::from_cartesian_pos_vel(pos_vel, k.epoch, frame);

            Some(SatelliteOrbitalState {
                orbit,
                epoch: k.epoch,
                satellite: k.sv,
                maneuver: v.maneuver,
            })
        }))
    }

    /// Form a [SatelliteOrbitalAttitude] [Iterator].
    #[cfg(feature = "anise")]
    #[cfg_attr(docsrs, doc(cfg(feature = "anise")))]
    pub fn satellites_attitude_iter(
        &self,
        almanac: Almanac,
        frame: Frame,
        rx_orbit: Orbit,
    ) -> Box<dyn Iterator<Item = SatelliteOrbitalAttitude> + '_> {
        Box::new(self.satellites_orbit_iter(frame).filter_map(move |state| {
            if let Ok(azelrange) =
                almanac.azimuth_elevation_range_sez(rx_orbit, state.orbit, None, None)
            {
                Some(SatelliteOrbitalAttitude {
                    azelrange,
                    maneuver: state.maneuver,
                    satellite: state.satellite,
                })
            } else {
                None
            }
        }))
    }
}
