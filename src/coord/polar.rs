use crate::axis::Axis;

pub(crate) struct Polar {
    pub(crate) radius: f64,
    pub(crate) angle: f64,
    pub(crate) radial_axis: Option<Axis>,  // For radial axis (like in radar charts)
    pub(crate) angular_axis: Option<Axis>, // For angular axis
}

impl Polar {
    /// Creates a new instance with the specified radius and angle.
    /// 
    /// # Arguments
    /// 
    /// * `radius` - The radial distance from the origin
    /// * `angle` - The angular position in radians
    /// 
    /// # Returns
    /// 
    /// A new instance with the given radius and angle, with radial_axis and angular_axis set to None
    fn new(radius: f64, angle: f64) -> Self {
        Self {
            radius,
            angle,
            radial_axis: None,
            angular_axis: None,
        }
    }

    // Convert polar coordinates to Cartesian for SVG rendering
    fn to_cartesian(&self, center_x: f64, center_y: f64) -> (f64, f64) {
        let x = center_x + self.radius * self.angle.cos();
        let y = center_y + self.radius * self.angle.sin();
        (x, y)
    }

    // Create function that maps data to angle (0 to 2π)
    fn data_to_angle(&self, data_min: f64, data_max: f64) -> impl Fn(f64) -> f64 {
        move |value| {
            if (data_max - data_min).abs() < 1e-12 {
                0.0
            } else {
                2.0 * std::f64::consts::PI * (value - data_min) / (data_max - data_min)
            }
        }
    }

    // Create function that maps data to radius
    fn data_to_radius(&self, data_min: f64, data_max: f64, max_radius: f64) -> impl Fn(f64) -> f64 {
        move |value| {
            if (data_max - data_min).abs() < 1e-12 {
                0.0
            } else {
                max_radius * (value-data_min) / (data_max - data_min)
            }
        }
    }
}