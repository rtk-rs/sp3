mod decimation;
mod masking;
mod split;
mod substract;
mod timeshift;

use crate::prelude::SP3;
use qc_traits::Preprocessing;

impl Preprocessing for SP3 {}
