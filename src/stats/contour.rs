//! Iso-line extraction from a regular grid (marching squares).
//!
//! A contour — an "iso-line" — is the set of points where a scalar field `z`
//! equals a chosen level. Given the field sampled on a regular grid, the
//! marching-squares algorithm walks every cell, finds where the level crosses
//! its four edges, and connects those crossings with a short segment. The
//! segments are then chained into polylines.
//!
//! This module is deliberately general: it knows nothing about density,
//! elevation or temperature. Any regular grid of `x`, `y` and `z` values can be
//! turned into contours, which is why a single transform can serve contour
//! plots, density plots and any other iso-line chart.

use ahash::AHashMap;

/// One extracted polyline: a constant `level` and the ordered vertices that
/// trace it, in data coordinates.
pub(crate) struct ContourLine {
    pub level: f64,
    pub points: Vec<(f64, f64)>,
}

/// A point keyed by its exact bit pattern.
///
/// Crossing points on a shared cell edge are computed from the same two corner
/// values and the same level, so they are bit-identical. Keying on the bits lets
/// us join segments without an epsilon.
type PointKey = (u64, u64);

const fn key(p: (f64, f64)) -> PointKey {
    (p.0.to_bits(), p.1.to_bits())
}

/// Extracts iso-lines for every value in `levels`.
///
/// # Arguments
/// * `xs` — the grid's x coordinates, ascending.
/// * `ys` — the grid's y coordinates, ascending.
/// * `z` — the field, row-major: `z[j * xs.len() + i]` is the value at
///   `(xs[i], ys[j])`. Its length must be `xs.len() * ys.len()`.
/// * `levels` — the values to trace.
pub(crate) fn contours(xs: &[f64], ys: &[f64], z: &[f64], levels: &[f64]) -> Vec<ContourLine> {
    let nx = xs.len();
    let ny = ys.len();
    let mut out = Vec::new();

    if nx < 2 || ny < 2 || z.len() != nx * ny {
        return out;
    }

    for &level in levels {
        let mut segments: Vec<((f64, f64), (f64, f64))> = Vec::new();

        for j in 0..ny - 1 {
            for i in 0..nx - 1 {
                let z0 = z[j * nx + i];
                let z1 = z[j * nx + i + 1];
                let z2 = z[(j + 1) * nx + i + 1];
                let z3 = z[(j + 1) * nx + i];

                // Skip cells the field did not fill (an incomplete grid).
                if !(z0.is_finite() && z1.is_finite() && z2.is_finite() && z3.is_finite()) {
                    continue;
                }

                let corners = [
                    (xs[i], ys[j]),
                    (xs[i + 1], ys[j]),
                    (xs[i + 1], ys[j + 1]),
                    (xs[i], ys[j + 1]),
                ];
                march_cell(level, [z0, z1, z2, z3], corners, &mut segments);
            }
        }

        chain(level, segments, &mut out);
    }

    out
}

/// Emits the segment(s) a single cell contributes at `level`.
///
/// `edges` are indexed 0..3 as `c0-c1`, `c1-c2`, `c2-c3`, `c3-c0`; the returned
/// segment connects the two edges the level crosses.
fn march_cell(
    level: f64,
    z: [f64; 4],
    p: [(f64, f64); 4],
    segments: &mut Vec<((f64, f64), (f64, f64))>,
) {
    let mut case = 0u8;
    for (k, &zk) in z.iter().enumerate() {
        if zk >= level {
            case |= 1 << k;
        }
    }
    if case == 0 || case == 15 {
        return;
    }

    // Crossing point on each edge (only the crossed ones are used). The
    // interpolation direction is canonical (bottom-to-top on vertical edges,
    // left-to-right on horizontal ones) so that the two cells sharing an edge
    // compute the *same* floating-point point. Without this, `chain` keys on
    // exact bits and a one-bit rounding difference splits the line.
    let point = |k: usize| -> (f64, f64) {
        let (a, b) = match k {
            2 => (3, 2), // top edge: left -> right
            3 => (0, 3), // left edge: bottom -> top
            _ => (k, (k + 1) % 4),
        };
        interpolate(p[a], p[b], z[a], z[b], level)
    };

    // Connect the crossed edges. The table is symmetric: case `k` and its
    // complement `15 - k` trace the same line.
    let pairs: &[(usize, usize)] = match case {
        1 | 14 => &[(0, 3)],
        2 | 13 => &[(0, 1)],
        3 | 12 => &[(1, 3)],
        4 | 11 => &[(1, 2)],
        6 | 9 => &[(0, 2)],
        7 | 8 => &[(2, 3)],
        // Saddles: the centre decides whether the "above" corners connect.
        5 | 10 => {
            let centre = (z[0] + z[1] + z[2] + z[3]) / 4.0;
            let above_connect = centre >= level;
            match (case, above_connect) {
                (5, true) | (10, false) => &[(0, 1), (2, 3)],
                _ => &[(0, 3), (1, 2)],
            }
        }
        _ => &[],
    };

    for &(a, b) in pairs {
        segments.push((point(a), point(b)));
    }
}

/// Linear interpolation of the crossing point on edge `a-b`.
fn interpolate(a: (f64, f64), b: (f64, f64), za: f64, zb: f64, level: f64) -> (f64, f64) {
    let denom = zb - za;
    let t = if denom.abs() < f64::EPSILON {
        0.5
    } else {
        ((level - za) / denom).clamp(0.0, 1.0)
    };
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Joins the per-cell segments of one level into polylines.
///
/// Segments are walked end to end; a chain ends at a free end or when it closes
/// on itself. The result is a small number of long polylines instead of one
/// two-point path per cell.
fn chain(level: f64, segments: Vec<((f64, f64), (f64, f64))>, out: &mut Vec<ContourLine>) {
    if segments.is_empty() {
        return;
    }

    let mut adjacency: AHashMap<PointKey, Vec<usize>> = AHashMap::new();
    for (index, (a, b)) in segments.iter().enumerate() {
        adjacency.entry(key(*a)).or_default().push(index);
        adjacency.entry(key(*b)).or_default().push(index);
    }

    let mut used = vec![false; segments.len()];

    for start in 0..segments.len() {
        if used[start] {
            continue;
        }

        used[start] = true;
        let (a, b) = segments[start];

        // The seed segment can sit anywhere in a polyline, so both ends must be
        // followed. First walk *backwards* from `a` and collect the part that
        // lies before the seed; reversing it puts it in front. Missing this
        // step leaves an orphan tail (often a lone two-point segment) whenever
        // the seed is not an end segment.
        let mut head = Vec::new();
        let mut tip = a;
        while let Some(next) = adjacency
            .get(&key(tip))
            .and_then(|list| list.iter().copied().find(|&s| !used[s]))
        {
            used[next] = true;
            let (na, nb) = segments[next];
            let other = if key(na) == key(tip) { nb } else { na };
            head.push(other);
            tip = other;
        }
        head.reverse();

        // Then walk forwards from `b`.
        let mut points = head;
        points.push(a);
        points.push(b);
        let mut tip = b;
        while let Some(next) = adjacency
            .get(&key(tip))
            .and_then(|list| list.iter().copied().find(|&s| !used[s]))
        {
            used[next] = true;
            let (na, nb) = segments[next];
            let other = if key(na) == key(tip) { nb } else { na };
            points.push(other);
            tip = other;
        }

        if points.len() >= 2 {
            out.push(ContourLine { level, points });
        }
    }
}

#[cfg(test)]
mod chain_tests {
    use super::*;

    #[test]
    fn chain_extends_in_both_directions_from_a_middle_seed() {
        // The first segment is a middle piece of a longer polyline. Both the
        // part before it and the part after it must end up in one line.
        let segments = vec![
            ((1.0, 0.0), (1.0, 1.0)),  // middle seed
            ((1.0, -1.0), (1.0, 0.0)), // before the seed
            ((1.0, 1.0), (1.0, 2.0)),  // after the seed
        ];
        let mut out = Vec::new();
        chain(5.0, segments, &mut out);

        assert_eq!(out.len(), 1, "fragmented into {} lines", out.len());
        assert_eq!(out[0].points.len(), 4);
        assert_eq!(out[0].points.first().copied(), Some((1.0, -1.0)));
        assert_eq!(out[0].points.last().copied(), Some((1.0, 2.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(n: usize) -> (Vec<f64>, Vec<f64>) {
        let coords: Vec<f64> = (0..n).map(|i| i as f64).collect();
        (coords.clone(), coords)
    }

    #[test]
    fn circles_of_a_bowl_close() {
        // z = (x-4)^2 + (y-4)^2; a level is a circle around (4, 4).
        let (xs, ys) = grid(9);
        let mut z = Vec::new();
        for y in &ys {
            for x in &xs {
                z.push((x - 4.0).powi(2) + (y - 4.0).powi(2));
            }
        }
        let lines = contours(&xs, &ys, &z, &[4.0]);
        assert_eq!(lines.len(), 1, "one closed loop at this level");

        // Every vertex must sit on the circle of radius 2.
        for (x, y) in &lines[0].points {
            let r = ((x - 4.0).powi(2) + (y - 4.0).powi(2)).sqrt();
            assert!((r - 2.0).abs() < 0.2, "radius {r}");
        }
        // A closed loop: first and last point coincide.
        let first = lines[0].points.first().unwrap();
        let last = lines[0].points.last().unwrap();
        assert!((first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9);
    }

    #[test]
    fn a_linear_field_gives_straight_lines() {
        // z = x; level 2.5 is the vertical line x = 2.5.
        let (xs, ys) = grid(5);
        let mut z = Vec::new();
        for _y in &ys {
            for x in &xs {
                z.push(*x);
            }
        }
        let lines = contours(&xs, &ys, &z, &[2.5]);
        assert_eq!(lines.len(), 1);
        for (x, _y) in &lines[0].points {
            assert!((x - 2.5).abs() < 1e-9);
        }
    }

    #[test]
    fn extreme_levels_produce_nothing() {
        let (xs, ys) = grid(4);
        let z = vec![0.0; xs.len() * ys.len()];
        assert!(contours(&xs, &ys, &z, &[1.0]).is_empty());
    }

    /// One long contour must come out as a single polyline, never as a long
    /// line plus a scatter of two-point fragments. A diagonal is a good probe:
    /// it crosses many shared cell edges, and its first cell segment sits in
    /// the middle of the line rather than at an end.
    #[test]
    fn a_single_contour_is_not_fragmented() {
        let n = 21;
        let xs: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let ys = xs.clone();
        let mut z = Vec::new();
        for y in &ys {
            for x in &xs {
                z.push(x + y);
            }
        }

        let lines = contours(&xs, &ys, &z, &[20.5]);
        assert_eq!(lines.len(), 1, "fragmented into {} lines", lines.len());
        assert!(
            lines[0].points.len() >= 20,
            "only {} points; the line was truncated",
            lines[0].points.len()
        );
    }
}
