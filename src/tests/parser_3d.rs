//! SP3-D dedicated tests
#[cfg(test)]
mod test {
    use crate::prelude::*;
    use gnss_rs::prelude::Constellation;
    use std::path::PathBuf;
    use std::str::FromStr;

    #[test]
    fn example_txt() {
        let path = PathBuf::new()
            .join(env!("CARGO_MANIFEST_DIR"))
            .join("data/SP3")
            .join("D")
            .join("example.txt");

        let sp3 = SP3::from_file(&path);

        assert!(
            sp3.is_ok(),
            "failed to parse data/sp3d.txt: {:?}",
            sp3.err()
        );

        let sp3 = sp3.unwrap();

        assert_eq!(sp3.header.version, Version::D);
        assert_eq!(sp3.header.data_type, DataType::Position);

        assert_eq!(
            sp3.first_epoch().unwrap(),
            Epoch::from_str("2019-10-27T00:00:00 GPST").unwrap()
        );

        assert_eq!(sp3.total_epochs(), 1);

        assert!(sp3.has_satellite_clock_offset());
        assert!(!sp3.has_satellite_clock_drift());
        assert!(!sp3.has_satellite_velocity());

        assert_eq!(sp3.header.coord_system, "IGS14");
        assert_eq!(sp3.header.orbit_type, OrbitType::FIT);
        assert_eq!(sp3.header.timescale, TimeScale::GPST);
        assert_eq!(sp3.header.constellation, Constellation::Mixed);
        assert_eq!(sp3.header.agency, "IGS");

        assert_eq!(sp3.header.week, 2077);
        assert_eq!(sp3.header.week_nanos, 0);

        assert_eq!(
            sp3.header.sampling_period,
            Duration::from_seconds(300.0_f64)
        );

        assert_eq!(sp3.header.mjd, 58783);
        assert_eq!(sp3.header.mjd_fraction, 0.0);

        let c01 = SV::from_str("C01").unwrap();
        let e01 = SV::from_str("E01").unwrap();
        let g01 = SV::from_str("G01").unwrap();
        let j01 = SV::from_str("J01").unwrap();
        let r01 = SV::from_str("R01").unwrap();

        let t0 = Epoch::from_str("2019-10-27T00:00:00 GPST").unwrap();

        for (epoch, sv, predicted, maneuver, position) in sp3.satellites_position_km_iter() {
            assert_eq!(epoch, t0);

            if sv == g01 {
                assert!(predicted);
                assert!(maneuver);
            } else {
                assert!(!predicted);
                assert!(!maneuver);
            }

            if sv == c01 {
                assert_eq!(
                    position,
                    (-32312.652253, 27060.656563, 205.195454),
                    "bad position data"
                );
            } else if sv == e01 {
                assert_eq!(
                    position,
                    (-15325.409333, 5781.454973, -24645.410980),
                    "bad position data"
                );
            } else if sv == g01 {
                assert_eq!(
                    position,
                    (-22335.782004, -14656.280389, -1218.238499),
                    "bad position data"
                );
            } else if sv == j01 {
                assert_eq!(
                    position,
                    (-30616.656355, 26707.752269, 16227.934145),
                    "bad position data"
                );
            } else if sv == r01 {
                assert_eq!(
                    position,
                    (15684.717752, -12408.390324, -15847.221180),
                    "bad position data"
                );
            }
        }

        for (epoch, sv, clock) in sp3.satellites_clock_offset_sec_iter() {
            assert_eq!(epoch, t0);

            if sv == c01 {
                assert!(
                    (clock - 63.035497E-6).abs() < 1E-9,
                    "bad clock data: {}",
                    clock
                );
            } else if sv == e01 {
                assert!(
                    (clock - -718.927492E-6).abs() < 1E-9,
                    "bad clock data: {}",
                    clock
                );
            } else if sv == g01 {
                assert!(
                    (clock - -176.397152E-6).abs() < 1E-9,
                    "bad clock data: {}",
                    clock
                );
            } else if sv == j01 {
                assert!(
                    (clock - -336.145158E-6).abs() < 1E-9,
                    "bad clock data: {}",
                    clock
                );
            } else if sv == r01 {
                assert!(
                    (clock - 51.759894E-6).abs() < 1E-9,
                    "bad clock data: {}",
                    clock
                );
            }
        }

        assert_eq!(
            sp3.comments.len(),
            4,
            "failed to parse files comment correctly"
        );

        assert_eq!(
            sp3.comments,
            vec![
                "PCV:IGS14_2074 OL/AL:FES2004  NONE     YN CLK:CoN ORB:CoN",
                "THIS EXAMPLE OF SP3 FILE IS PART OF THE gLAB TOOL SUITE",
                "FILE PREPARED BY: MOWEN LI",
                "PLEASE EMAIL ANY COMMENT OR REQUEST TO glab.gage @upc.edu",
            ],
        );
    }
}
