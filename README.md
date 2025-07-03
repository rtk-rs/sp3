# SP3

[![crates.io](https://img.shields.io/crates/v/sp3.svg)](https://crates.io/crates/sp3)
[![Rust](https://github.com/rtk-rs/sp3/actions/workflows/rust.yml/badge.svg)](https://github.com/rtk-rs/sp3/actions/workflows/rust.yml)
[![Rust](https://github.com/rtk-rs/sp3/actions/workflows/daily.yml/badge.svg)](https://github.com/rtk-rs/sp3/actions/workflows/daily.yml)
[![crates.io](https://docs.rs/sp3/badge.svg)](https://docs.rs/sp3/)
[![crates.io](https://img.shields.io/crates/d/sp3.svg)](https://crates.io/crates/sp3)

[![MRSV](https://img.shields.io/badge/MSRV-1.82.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.82.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/rtk-rs/sp3/blob/main/LICENSE)

SP3 Precise GNSS Orbit file parsing, analysis & production. 

This file format is specifid by the [International GNSS Service (IGS)](https://igs.org/products/#orbits_clocks).

NB: this library only supports revisions C & D (latest).

## SP3 files content

SP3 files provide satellite position vector with a high precision (+/- 1mm),
which is compatible with high precision geodesy.

Sometimes SP3 files may provide velocity vectors, satellite clock offsets
or satellite clock drifts as well.

## Getting started

```toml
[dependencies]
sp3 = "1"
```

Parse an SP3 file

```rust
use sp3::prelude::*;
use std::path::PathBuf;
use std::str::FromStr;
    
let path = PathBuf::new()
    .join(env!("CARGO_MANIFEST_DIR"))
    .join("data/SP3")
    .join("C")
    .join("ESA0OPSRAP_20232390000_01D_15M_ORB.SP3.gz");

let sp3 = SP3::from_gzip_file(&path).unwrap();

assert_eq!(sp3.header.version, Version::C);
assert_eq!(sp3.header.data_type, DataType::Position);

let t0 = sp3.first_epoch().unwrap();

assert_eq!(
    t0,
    Epoch::from_str("2023-08-27T00:00:00 GPST").unwrap()
);

assert_eq!(sp3.total_epochs(), 96);
assert_eq!(sp3.header.agency, "ESOC");

// All coordinates expressed in the following system
assert_eq!(sp3.header.coord_system, "ITRF2");

// Orbit type used in fitting process
assert_eq!(sp3.header.orbit_type, OrbitType::BHN);

// This means all temporal information is expressed in this [TimeScale]
assert_eq!(sp3.header.timescale, TimeScale::GPST);

// This means several constellations are to be found
assert_eq!(sp3.header.constellation, Constellation::Mixed);

// Week counter, in given [TimeScale]
assert_eq!(sp3.header.week, 2277);
assert_eq!(sp3.header.week_nanos, 0);

assert_eq!(sp3.header.sampling_period, Duration::from_seconds(900.0_f64));

// Data exploitation
for (epoch, sv, predicted, maneuver, (x_km_ecef, y_km_ecef, z_km_ecef)) in sp3.satellites_position_km_iter() {
    
    if predicted {
        // results from prediction algorithm, not a fit
    } else {
        // results from a fit algorithm, not a prediction
    }

    if maneuver {
        // sv being maneuvered: not suited for precise navigation.
    }
}

// Data exploitation
for (epoch, sv, clock) in sp3.satellites_clock_offset_sec_iter() {

}

// Dump to file
sp3.to_file("/tmp/test.txt")
    .unwrap();

let attributes = sp3.prod_attributes
    .expect("exists for files that follow standards conventions!");

// "ESA0OPSRAP_20232390000_01D_15M_ORB.SP3.gz
assert_eq!(attributes.agency, "ESA");
assert_eq!(attributes.batch_id, 0);
assert_eq!(attributes.availability, Availability::Rapid);
assert_eq!(attributes.release_date.year, 2023);
assert_eq!(attributes.release_date.doy, 239);
assert_eq!(attributes.release_period, ReleasePeriod::Daily);
assert_eq!(attributes.sampling_period, sp3.header.sampling_period);
assert!(attributes.gzip_compressed);
```

## Lib features

This library comes with a few features

- `flate2` will enable direct support of Gzip compressed SP3 files
- `serde` will unlock internal structure serdes ops
- `anise` feature will unlock Elevation and Azimuth attitudes (heaviest dependency).
- `qc` option will unlock basic file management options like Merge(A, B) or Split (timewise)
- `processing` relies on `qc` and unlocks file preprocessing, like resampling and data masking
- interpolation methods are proposed by default (they do not involve other dependencies)

## Default features

This library is shipped with `flate2` support (gzip compressed SP3 files) by default.

## Main dependencies

This library relies on `Nyx-Space/Hifitime` at all times.

The `Nyx-Space/ANISE` feature is the heaviest library option. 

## Satellite attitude interpolation

Satellite (SV) attitude interpolation is a major topic in SP3 processing.  
Typically, SP3 data has to be matched (in time) with other data which requires interpolation.
In order to preserve the quality of the SP3 fit, it is recommended to use a high order Lagrangian
interpolation. This library provides SP3 interpolation as is, because it does not involve external libraries:

- `[SP3.satellite_position_interp()]` will design the interpolation kernel
to which you can apply your custom interpolation function

- `[SP3.satellite_lagrangian_position_interp()]` will apply the Lagrangian interpolatation
method, typically used in geodetic processing piplines, at the desired interpolation order.

- `[SP3.satellite_lagrangian_position_interp_x11()]` applies the Lagrangian interpolation
method with a order of 11, which is typically used to preserve SP3 precision

- `[SP3.satellite_lagrangian_position_interp_x17()]` applies the Lagrangian interpolation
method with a order of 17, which is way more than enough and should be used in processing
pipelines where processing speed and resource consumption is not an issue. 

:warning: our interpolation method does not support even interpolation orders.
The extracted kernel is therefore:

- `tmin = (N +1)/2 * τ`
- `tmax =  T(n-1) - (N +1)/2 * τ`

with `τ` the sampling internval, `T(n-1)` the last epoch provided.

```rust
use sp3::prelude::*;
use std::str::FromStr;
use std::path::PathBuf;

let path = PathBuf::new()
    .join(env!("CARGO_MANIFEST_DIR"))
    .join("data/SP3")
    .join("C")
    .join("ESA0OPSRAP_20232390000_01D_15M_ORB.SP3.gz");

let sp3 = SP3::from_gzip_file(&path)
    .unwrap();

let g01 = SV::from_str("G01").unwrap();

// first epoch in this file
let t0 = Epoch::from_str("2023-08-27T00:00:00 GPST")
    .unwrap();

// after 7th epoch we can interpolate by x11 
let t7 = Epoch::from_str("2023-08-27T00:00:00 GPST")
    .unwrap();

let interpolated = sp3.satellite_position_lagrangian_11_interpolation(g01, t0);
assert!(interpolated.is_none(), "too early in this file");

let interpolated = sp3.satellite_position_lagrangian_17_interpolation(g01, t0);
assert!(interpolated.is_none(), "too early in this file");
```

## Satellite clock interpolation

Although it is feasible to interpolate the clock state, it is not recommended to do so.
If your processing pipeline requires to interpolate the clock state, you should most likely
redesign it or reconsider your dataset.

Clock interpolation should be restricted to short intervals (like 30s at most).

We propose a similar API for clock interpolation as the attitude interpolation.

## QC: File Merging

Merge two files together, for example to create a context spanning 48 hours

```rust
use std::path::PathBuf;
use sp3::prelude::{SP3, Merge};

let folder = PathBuf::new()
    .join(env!("CARGO_MANIFEST_DIR"))
    .join("data/SP3")
    .join("C");

let sp3_a = folder.clone()
    .join("ESA0OPSRAP_20232390000_01D_15M_ORB.SP3.gz");

let sp3_b = folder.clone()
    .join("ESA0OPSULT_20232320600_02D_15M_ORB.SP3.gz");

let sp3_a = SP3::from_gzip_file(&sp3_a)
    .unwrap();

let sp3_b = SP3::from_gzip_file(&sp3_b)
    .unwrap();

let sp3 = sp3_a.merge(&sp3_b);
assert!(sp3.is_ok());
```

## QC: Timescale Transposition

Use the `Timeshift` trait to transpose the SP3 into other timescales. Coarse
and precise transpositions are both supported. The precise transposition method
requires a correction database.

```rust
use sp3::prelude::*;
use std::path::PathBuf;

let path = PathBuf::new()
    .join(env!("CARGO_MANIFEST_DIR"))
    .join("data/SP3")
    .join("C")
    .join("ESA0OPSRAP_20232390000_01D_15M_ORB.SP3.gz");

// Typical GPST SP3
let gpst_sp3 = SP3::from_gzip_file(&path)
    .unwrap();

// // Transpose to GST
// let gst_sp3 = gpst_sp3.timeshift(TimeScale::GST);

// Dump as GST file
// gst_sp3.to_file("/tmp/gst.txt")
//    .unwrap();
```

## License

The SP3 library is released under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.  
The `nyx` feature is released under [AGPL v3](https://www.gnu.org/licenses/agpl-3.0.fr.html).
