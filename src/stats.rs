//! Reusable statistics, with no plotting logic.
//!
//! These are the numerical kernels the transforms are built from:
//! one- and two-dimensional kernel density estimation, iso-line extraction by
//! marching squares, and LOESS smoothing. They operate on plain numbers, which
//! keeps them easy to test and lets several transforms share the same maths.

pub(crate) mod contour;
pub(crate) mod kde;
pub(crate) mod kde2d;
pub(crate) mod stat_loess;
