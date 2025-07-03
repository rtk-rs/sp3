/*
 * Authors: Guillaume W. Bres <guillaume.bressaix@gmail.com> et al.
 * (cf. https://github.com/rtk-rs/sp3/graphs/contributors)
 * This framework is shipped under Mozilla Public V2 license.
 *
 * Documentation: https://github.com/rtk-rs/sp3
 *
 * The nyx feature is released under AGPLv3
 * Copyright (C) 2021-onward Christopher Rabotin <christopher.rabotin@gmail.com> et al. (cf. AUTHORS.md)
 * Documentation: https://nyxspace.com/
 */
use crate::prelude::{Duration, Epoch, SP3, SV};

use anise::{
    constants::{
        celestial_objects::{MOON, SUN},
        frames::{EARTH_J2000, MOON_J2000},
    },
    prelude::Almanac,
};

use std::{collections::BTreeMap, sync::Arc};

use nyx_space::{
    dynamics::{OrbitalDynamics, SolarPressure, SpacecraftDynamics},
    od::{kalman::KalmanVariant, SpacecraftKalmanOD},
    propagators::{IntegratorOptions, Propagator},
};

impl SP3 {
    /// Propagate this [SP3] into the future, returning a new extended [SP3].
    /// Refer to [Self::propagate_mut] for more information.
    pub fn propagate(&self, almanac: Arc<Almanac>, prediction_duration: Duration) -> Self {
        let mut s = self.clone();
        s.propagate_mut(almanac, prediction_duration);
        s
    }

    /// Propagate this (entire) [SP3] in the future, using [Propagator::dp78]
    /// and mutable access.
    ///
    /// ## Input
    /// - almanac: [Almanac]
    /// - prediction_duration: [Duration] of the prediction
    pub fn propagate_mut(&mut self, almanac: Arc<Almanac>, prediction_duration: Duration) {
        let opts = IntegratorOptions::with_fixed_step(self.header.sampling_period);

        let orbital_model = OrbitalDynamics::point_masses(vec![MOON, SUN]);

        let srp_model = SolarPressure::new(vec![EARTH_J2000, MOON_J2000], almanac.clone())
            .unwrap_or_else(|e| {
                // TODO replace with proper error
                panic!("failed to build solar pressure model: {}", e);
            });

        let dynamics = SpacecraftDynamics::from_model(orbital_model, srp_model);

        // Propagates each satellite, arming the propagator using last described precise position.
        // There should be better approach by taking all the data set into account, reducing the propagtion error.
        let last_epoch = self.last_epoch().expect("empty data set"); // TODO replace with proper error

        let final_states = self
            .data
            .iter()
            .filter(|(k, v)| k.epoch == last_epoch)
            .collect::<Vec<_>>();

        // Create a propagator that uses the same model
        let setup = Propagator::dp78(dynamics, opts);

        for (k, v) in final_states.iter() {
            // Deploy a process
            let mut odp = SpacecraftKalmanOD::new(
                setup.clone(),
                KalmanVariant::DeviationTracking,
                None,
                BTreeMap::new(),
                almanac.clone(),
            );
        }
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{Duration, Epoch, Split, SP3};
    use anise::prelude::Almanac;
    use std::str::FromStr;
    use std::sync::Arc;

    #[test]
    fn test_nyx_propagator() {
        let almanac = Arc::new(Almanac::until_2035().unwrap());

        // entire setup
        let parsed =
            SP3::from_gzip_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz").unwrap();

        // grab first 12 hours, propagate last 12 hours,
        let noon = Epoch::from_str("2020-06-25T12:00:00 GPST").unwrap();
        let (sp3_morning, _) = parsed.split(noon);

        let predicted = sp3_morning.propagate(almanac, Duration::from_hours(12.0));

        // compare
        let residuals = predicted.substract(&parsed);
    }
}
