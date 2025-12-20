use crate::axis::Axis;
use crate::coord::Scale;

#[derive(Clone)]
pub(crate) struct Cartesian {
    pub(crate) x_axis: Axis,
    pub(crate) y_axis: Axis,
    pub(crate) x_axis_padding_min: f64, // 0.0-1.0
    pub(crate) x_axis_padding_max: f64, // 0.0-1.0
    pub(crate) y_axis_padding_min: f64, // 0.0-1.0
    pub(crate) y_axis_padding_max: f64, // 0.0-1.0
}

impl Cartesian {
    /// Creates a new instance with the specified x and y axes.
    ///
    /// # Arguments
    ///
    /// * `x_axis` - The axis to be used for the x dimension
    /// * `y_axis` - The axis to be used for the y dimension
    ///
    /// # Returns
    ///
    /// A new instance containing the provided x and y axes
    pub(crate) fn new(
        x_axis: Axis,
        y_axis: Axis,
        x_axis_padding_min: f64,
        x_axis_padding_max: f64,
        y_axis_padding_min: f64,
        y_axis_padding_max: f64,
    ) -> Self {
        Self {
            x_axis,
            y_axis,
            x_axis_padding_min,
            x_axis_padding_max,
            y_axis_padding_min,
            y_axis_padding_max,
        }
    }

    // Creates a function that maps x-axis data coordinates to horizontal pixel coordinates
    pub(crate) fn x_data_to_horizontal_pixels(
        &self,
        draw_start: f64,
        plot_width: f64,
    ) -> impl Fn(f64) -> f64 {
        let ticks = self.x_axis.automatic_ticks.clone();
        move |x| {
            // For discrete scales, we need to map indices properly
            if matches!(self.x_axis.scale, Scale::Discrete) {
                // Map discrete data index to pixel position with padding
                let num_categories = ticks.ticks.len();
                if num_categories == 1 {
                    draw_start + plot_width / 2.0
                } else {
                    let index = x; // x is already the index for discrete data
                    // Add padding on each side
                    let padded_min = -self.x_axis_padding_min;
                    let padded_max = (num_categories - 1) as f64 + self.x_axis_padding_max;
                    let padded_range = padded_max - padded_min;
                    draw_start + (index - padded_min) / padded_range * plot_width
                }
            } else {
                // For continuous scales, map data range to pixel range
                let min = ticks.ticks.first().unwrap().position;
                let max = ticks.ticks.last().unwrap().position;

                // Adding padding on both sides
                let max_ticks = ticks.ticks.len(); // ticks number
                let padding_space_min = plot_width * (self.x_axis_padding_min / max_ticks as f64);
                let padding_space_max = plot_width * (self.x_axis_padding_max / max_ticks as f64);
                let padded_width = plot_width - padding_space_min - padding_space_max;

                draw_start + padding_space_min + (x - min) / (max - min) * padded_width
            }
        }
    }

    // Creates a function that maps y-axis data coordinates to vertical pixel coordinates
    pub(crate) fn y_data_to_vertical_pixels(
        &self,
        draw_start: f64,
        plot_height: f64,
    ) -> impl Fn(f64) -> f64 {
        let ticks = self.y_axis.automatic_ticks.clone();
        move |y| {
            // For discrete scales
            if matches!(self.y_axis.scale, Scale::Discrete) {
                let num_categories = ticks.ticks.len();
                if num_categories == 1 {
                    draw_start + plot_height / 2.0
                } else {
                    let index = y; // y is already the index for discrete data
                    // Add padding on each side
                    let padded_min = -self.y_axis_padding_min;
                    let padded_max = (num_categories - 1) as f64 + self.y_axis_padding_max;
                    let padded_range = padded_max - padded_min;
                    // Note: Pixel y-coordinates increase downward, so we invert
                    draw_start + plot_height - (index - padded_min) / padded_range * plot_height
                }
            } else {
                // For continuous scales
                let min = ticks.ticks.first().unwrap().position;
                let max = ticks.ticks.last().unwrap().position;

                // Adding padding on both sides
                let max_ticks = ticks.ticks.len(); // ticks number
                let padding_space_min = plot_height * (self.y_axis_padding_min / max_ticks as f64);
                let padding_space_max = plot_height * (self.y_axis_padding_max / max_ticks as f64);
                let padded_height = plot_height - padding_space_min - padding_space_max;

                // Note: Pixel y-coordinates increase downward, so we invert
                draw_start + plot_height
                    - padding_space_min
                    - (y - min) / (max - min) * padded_height
            }
        }
    }

    // Creates a function that maps x-axis data coordinates to vertical pixel coordinates
    pub(crate) fn x_data_to_vertical_pixels(
        &self,
        draw_start: f64,
        plot_height: f64,
    ) -> impl Fn(f64) -> f64 {
        let ticks = self.x_axis.automatic_ticks.clone();
        move |x| {
            // For discrete scales
            if matches!(self.x_axis.scale, Scale::Discrete) {
                let num_categories = ticks.ticks.len();
                if num_categories == 1 {
                    draw_start + plot_height / 2.0
                } else {
                    let index = x; // x is already the index for discrete data
                    // Add padding on each side
                    let padded_min = -self.x_axis_padding_min;
                    let padded_max = (num_categories - 1) as f64 + self.x_axis_padding_max;
                    let padded_range = padded_max - padded_min;
                    // Note: For vertical display, higher values should appear higher (SVG y increases downward)
                    draw_start + plot_height - (index - padded_min) / padded_range * plot_height
                }
            } else {
                // For continuous scales
                let min = ticks.ticks.first().unwrap().position;
                let max = ticks.ticks.last().unwrap().position;

                // Adding padding on both sides
                let max_ticks = ticks.ticks.len(); // ticks number
                let padding_space_min = plot_height * (self.x_axis_padding_min / max_ticks as f64);
                let padding_space_max = plot_height * (self.x_axis_padding_max / max_ticks as f64);
                let padded_height = plot_height - padding_space_min - padding_space_max;

                // Note: For vertical display, higher values should appear higher (SVG y increases downward)
                draw_start + plot_height
                    - padding_space_min
                    - (x - min) / (max - min) * padded_height
            }
        }
    }

    // Creates a function that maps y-axis data coordinates to horizontal pixel coordinates
    pub(crate) fn y_data_to_horizontal_pixels(
        &self,
        draw_start: f64,
        plot_width: f64,
    ) -> impl Fn(f64) -> f64 {
        let ticks = self.y_axis.automatic_ticks.clone();
        move |y| {
            // For discrete scales
            if matches!(self.y_axis.scale, Scale::Discrete) {
                let num_categories = ticks.ticks.len();
                if num_categories == 1 {
                    draw_start + plot_width / 2.0
                } else {
                    let index = y; // y is already the index for discrete data
                    // Add padding on each side
                    let padded_min = -self.y_axis_padding_min;
                    let padded_max = (num_categories - 1) as f64 + self.y_axis_padding_max;
                    let padded_range = padded_max - padded_min;
                    // For horizontal display, we don't invert (higher values go to the right)
                    draw_start + (index - padded_min) / padded_range * plot_width
                }
            } else {
                // For continuous scales
                let min = ticks.ticks.first().unwrap().position;
                let max = ticks.ticks.last().unwrap().position;

                // Adding padding on both sides
                let max_ticks = ticks.ticks.len(); // ticks number
                let padding_space_min = plot_width * (self.y_axis_padding_min / max_ticks as f64);
                let padding_space_max = plot_width * (self.y_axis_padding_max / max_ticks as f64);
                let padded_width = plot_width - padding_space_min - padding_space_max;

                // For horizontal display, we don't invert (higher values go to the right)
                draw_start + padding_space_min + (y - min) / (max - min) * padded_width
            }
        }
    }
}
