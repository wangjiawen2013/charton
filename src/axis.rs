use crate::coord::Scale;
use crate::error::ChartonError;

#[derive(Debug, Clone)]
pub(crate) struct Tick {
    pub(crate) position: f64,
    pub(crate) label: String,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Ticks {
    pub(crate) ticks: Vec<Tick>,
}

#[derive(Debug, Clone)]
pub(crate) struct Axis {
    pub(crate) label: String,
    pub(crate) scale: Scale,
    // Used for mapping (x_mapper and y_mapper)
    pub(crate) automatic_ticks: Ticks,
    // Used for rendering
    pub(crate) explicit_ticks: Ticks,
}

impl Axis {
    pub(crate) fn new(label: impl Into<String>, scale: Scale) -> Self {
        Self {
            label: label.into(),
            scale,
            automatic_ticks: Ticks::default(),
            explicit_ticks: Ticks::default(),
        }
    }

    pub(crate) fn with_explicit_ticks(mut self, ticks: Vec<(f64, String)>) -> Self {
        let tick_objects: Vec<Tick> = ticks
            .into_iter()
            .map(|(position, label)| Tick { position, label })
            .collect();
        self.explicit_ticks = Ticks {
            ticks: tick_objects,
        };

        self
    }

    pub(crate) fn compute_discrete_ticks(
        &mut self,
        labels: Vec<String>,
    ) -> Result<(), ChartonError> {
        let ticks: Vec<Tick> = labels
            .into_iter()
            .enumerate()
            .map(|(index, label)| Tick {
                position: index as f64,
                label,
            })
            .collect();

        self.automatic_ticks = Ticks {
            ticks: ticks.clone(),
        };
        self.explicit_ticks = self.automatic_ticks.clone();
        Ok(())
    }

    pub(crate) fn compute_continuous_ticks(
        &mut self,
        min: f64,
        max: f64,
        pixels: u32,
    ) -> Result<(), ChartonError> {
        // Approximately 1 tick per 70 pixels, with a minimum of 2 ticks
        let max_ticks = (pixels / 70).max(2);
        self.automatic_ticks = self.compute_ticks(min, max, max_ticks as usize, &self.scale)?;
        self.explicit_ticks = self.automatic_ticks.clone();
        Ok(())
    }

    pub(crate) fn compute_ticks(
        &self,
        min: f64,
        max: f64,
        max_ticks: usize,
        scale: &Scale,
    ) -> Result<Ticks, ChartonError> {
        let mut tick_values: Vec<f64> = Vec::new();
        let max_ticks_f64 = max_ticks.max(2) as f64;

        // Floating point tolerance factor
        const EPSILON_FACTOR: f64 = 1e-9;
        // Maximum iterations to prevent infinite loops
        const MAX_ITERATIONS: usize = 10000;

        // Validate input parameters
        if !min.is_finite() || !max.is_finite() {
            return Err(ChartonError::Data("Invalid min/max values".to_string()));
        }

        if min > max {
            return Err(ChartonError::Data(
                "Min value must be less than or equal to max value".to_string(),
            ));
        }

        match scale {
            Scale::Linear => {
                let range = max - min;

                if range.abs() < 1e-12 {
                    // Extremely small or zero range: special handling
                    let magnitude = 10f64.powf(min.abs().log10().floor().max(0.0));
                    let step = if magnitude > 1e-12 { magnitude } else { 1.0 };

                    // Ensure at least two ticks for mapping
                    let start = (min / step).floor() * step;
                    tick_values.push(start);
                    tick_values.push(start + step);
                } else {
                    // 1. Determine "nice" step size
                    let rough_step = range / max_ticks_f64;
                    let exp = 10f64.powf(rough_step.log10().floor());
                    let f = rough_step / exp;
                    // Nice numbers: 1, 2, 5, 10
                    let nice = if f < 1.5 {
                        1.0
                    } else if f < 3.0 {
                        2.0
                    } else if f < 7.0 {
                        5.0
                    } else {
                        10.0
                    };
                    let step = nice * exp;
                    let tolerance = step * EPSILON_FACTOR;

                    // Validate step to prevent infinite loops
                    if step <= 0.0 || !step.is_finite() {
                        return Err(ChartonError::Data(
                            "Invalid step size calculated".to_string(),
                        ));
                    }

                    // 2. Determine nice start and end tick values (ensuring min and max are covered)
                    let mut start = (min / step).floor() * step;
                    let mut end = (max / step).ceil() * step;

                    // 3. Tighten bounds to reduce unnecessary padding
                    // Try to tighten end: if end-step still covers max, then tighten end
                    if end - step >= max + tolerance && (end - step).abs() > tolerance {
                        end -= step;
                    }

                    // Try to tighten start: if start+step is still covered by min, then tighten start
                    if start + step <= min - tolerance && (start + step).abs() > tolerance {
                        start += step;
                    }

                    // 4. Generate tick values
                    let mut pos = start;
                    let mut iterations = 0;

                    while pos <= end + tolerance && iterations < MAX_ITERATIONS {
                        // Reduce floating point noise
                        let rounded = (pos / tolerance).round() * tolerance;

                        // Avoid duplicates
                        if tick_values
                            .last()
                            .is_none_or(|&p| (p - rounded).abs() > tolerance)
                        {
                            tick_values.push(rounded);
                        }

                        let old_pos = pos;
                        pos += step;
                        iterations += 1;

                        // Break if no progress or invalid value
                        if (pos - old_pos).abs() < tolerance * 0.1 || !pos.is_finite() {
                            break;
                        }
                    }

                    // If after tightening we have only one tick, restore to non-tightened range
                    if tick_values.len() <= 1 {
                        tick_values.clear();
                        start = (min / step).floor() * step;
                        end = (max / step).ceil() * step;

                        let mut pos = start;
                        let mut iterations = 0;

                        while pos <= end + tolerance && iterations < MAX_ITERATIONS {
                            let rounded = (pos / tolerance).round() * tolerance;
                            if tick_values
                                .last()
                                .is_none_or(|&p| (p - rounded).abs() > tolerance)
                            {
                                tick_values.push(rounded);
                            }

                            let old_pos = pos;
                            pos += step;
                            iterations += 1;

                            // Break if no progress or invalid value
                            if (pos - old_pos).abs() < tolerance * 0.1 || !pos.is_finite() {
                                break;
                            }
                        }
                    }
                }
            }
            Scale::Log => {
                if min <= 0.0 {
                    return Err(ChartonError::Data(
                        "Log scale requires positive values".to_string(),
                    ));
                }

                // Validate log inputs
                if !min.is_finite() || !max.is_finite() {
                    return Err(ChartonError::Data(
                        "Invalid values for log scale".to_string(),
                    ));
                }

                // Traditional Log Scale: major ticks are powers of 10
                let log_min = min.log10();
                let log_max = max.log10();
                let major_step = 1.0;

                // Protect against invalid log values
                if !log_min.is_finite() || !log_max.is_finite() {
                    return Err(ChartonError::Data(
                        "Invalid log values calculated".to_string(),
                    ));
                }

                let log_start = log_min.floor();
                let log_end = log_max.ceil();

                let mut pos = log_start;
                let tolerance = 1e-9;
                let mut iterations = 0;

                while pos <= log_end + tolerance && iterations < MAX_ITERATIONS {
                    // Insert major tick (10^N)
                    tick_values.push(pos);
                    pos += major_step;
                    iterations += 1;
                }

                // Add minor ticks (10^N * 2, 3, 5 etc.) based on max ticks and range
                // Only add minor ticks when range is small (e.g., less than 3 decades)
                if log_end - log_start < max_ticks_f64 / 3.0 {
                    let mut extra_ticks: Vec<f64> = Vec::new();
                    // Consider 2 and 5 as important minor ticks
                    let minor_steps = [2.0f64.log10(), 5.0f64.log10()];

                    for &log_v in tick_values.iter() {
                        for &minor_log_step in minor_steps.iter() {
                            let minor_log_pos = log_v + minor_log_step;
                            if minor_log_pos > log_min - tolerance
                                && minor_log_pos < log_max + tolerance
                            {
                                extra_ticks.push(minor_log_pos);
                            }
                        }
                    }
                    tick_values.extend(extra_ticks);
                    tick_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    tick_values.dedup_by(|a, b| (*a - *b).abs() < tolerance);
                }
            }
            Scale::Discrete => {
                return Err(ChartonError::Scale(
                    "Cannot compute continuous ticks for discrete scale.".to_string(),
                ));
            }
        }

        // --- Label formatting logic ---

        // Decide label format: scientific or regular
        let abs_max = min.abs().max(max.abs());
        let use_sci = abs_max >= 1e6 || (abs_max < 1e-3 && abs_max > 0.0);

        // Determine uniform decimal count for regular formatting
        let step_for_dec = if tick_values.len() >= 2 {
            // Calculate step in data space
            if *scale == Scale::Log {
                (10f64.powf(tick_values[1]) - 10f64.powf(tick_values[0]))
                    .abs()
                    .max(1e-30)
            } else {
                (tick_values[1] - tick_values[0]).abs().max(1e-30)
            }
        } else {
            // Single tick fallback: based on magnitude of values
            let refv = min.abs().max(1.0);
            (refv * 0.1).max(1e-30)
        };

        let step_in_data_space = step_for_dec;

        let decimals = if use_sci {
            1usize // Scientific notation typically fixed at 1 decimal place
        } else {
            // ceil(-log10(step)), capped at [0, 6]
            let d = (-step_in_data_space.log10()).ceil() as isize;
            let d = if d < 0 { 0 } else { d as usize };
            d.min(6)
        };

        // Format tick values
        let ticks = tick_values
            .into_iter()
            .map(|v| {
                let label = match scale {
                    Scale::Log => {
                        // For Log scale, convert back to data space from log space
                        let data_value = 10f64.powf(v);
                        // Fix negative zero issue
                        let formatted_data_value = if data_value.abs() < 1e-12 {
                            0.0
                        } else {
                            data_value
                        };

                        if use_sci {
                            format!("{:.1e}", formatted_data_value)
                        } else {
                            // If it's a major tick (power of 10), display as integer
                            if (formatted_data_value.log10().round() - formatted_data_value.log10())
                                .abs()
                                < 1e-9
                            {
                                format!("{:.0}", formatted_data_value)
                            } else {
                                format!("{:.prec$}", formatted_data_value, prec = decimals)
                            }
                        }
                    }
                    _ => {
                        // For Linear scale, use the value directly
                        let formatted_value = if v.abs() < 1e-12 { 0.0 } else { v };

                        if use_sci {
                            format!("{:.1e}", formatted_value)
                        } else {
                            format!("{:.prec$}", formatted_value, prec = decimals)
                        }
                    }
                };
                Tick { position: v, label }
            })
            .collect();

        Ok(Ticks { ticks })
    }
}
