//! Reusable statistics, with no plotting logic.
//!
//! These are the numerical kernels the transforms are built from:
//! one- and two-dimensional kernel density estimation, iso-line extraction by
//! marching squares, and binning. They operate on plain numbers, which keeps
//! them easy to test and lets several transforms share the same maths.

#[allow(dead_code)]
pub(crate) mod contour;
#[allow(dead_code)]
pub(crate) mod kde;
#[allow(dead_code)]
pub(crate) mod kde2d;
#[allow(dead_code)]
pub(crate) mod stat_binning;
pub(crate) mod stat_loess;
