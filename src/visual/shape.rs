/// Represents different geometric shapes that can be used for data points in visualizations.
///
/// This enum defines various point shapes that can be used to represent data points
/// in scatter plots, line charts, and other visualization types. Each variant
/// corresponds to a distinct geometric shape that helps differentiate data series
/// or categories visually.
///
/// # Variants
///
/// * `Circle` - A circular point shape
/// * `Square` - A square point shape
/// * `Triangle` - A triangular point shape
/// * `Star` - A star-shaped point
/// * `Diamond` - A diamond-shaped point (rotated square)
/// * `Pentagon` - A five-sided polygonal point
/// * `Hexagon` - A six-sided polygonal point
/// * `Octagon` - An eight-sided polygonal point
///
/// # Examples
///
/// ```
/// use charton::visual::shape::PointShape;
///
/// let shape = PointShape::Circle;
/// ```
#[derive(Clone, Debug)]
pub enum PointShape {
    Circle,
    Square,
    Triangle,
    Star,
    Diamond,
    Pentagon,
    Hexagon,
    Octagon,
}

impl PointShape {
    /// Shapes used for legend and mapping
    pub(crate) const LEGEND_SHAPES: &'static [PointShape] = &[
        PointShape::Circle,
        PointShape::Square,
        PointShape::Triangle,
        PointShape::Star,
        PointShape::Diamond,
        PointShape::Pentagon,
        PointShape::Hexagon,
        PointShape::Octagon,
    ];
}
