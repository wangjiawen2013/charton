# Coordinate Systems

If Scales define the mathematical mapping of data, the Coordinate System defines the physical space where that mapping is realized. It determines how the $(x, y)$ pairs from our scales are positioned and distorted on the canvas to create different types of visualizations.

## The Role of Coordinates

The coordinate system is the final arbiter of geometry. It takes normalized values (from 0 to 1) and translates them into physical coordinates. Its responsibilities include:

* Spatial Translation: Converting abstract ratios into pixel positions.
* Axis Orientation: Deciding if the X-axis runs horizontally or wraps around a circle.
* Visual Transformation: Handling effects like "Coord Flip" (swapping X and Y) or polar projections.
* Clipping: Determining if data points that fall outside the defined boundaries should be hidden.

## Cartesian Coordinates (`Cartesian2D`)

The Cartesian system is the most common coordinate system, mapping data onto a rectangular plane. 

* Linear Mapping: It directly maps normalized X to width and normalized Y to height.
* Coordinate Flipping: Charton supports a "Flipped" state. When enabled, the X-axis becomes vertical and the Y-axis becomes horizontal. This is a powerful way to turn a vertical Bar Chart into a horizontal one without changing the underlying data encoding.
* Layout Hints: The Cartesian system provides specific hints to the layout engine, such as suggesting that bars should occupy 50% of their available slot width by default to ensure readability.

## Polar Coordinates (`Polar`)

The Polar coordinate system maps data into a circular space, which is essential for radial visualizations like Pie charts, Donut charts, and Rose plots.

* Angle and Radius: 
    * The X dimension is mapped to the Angle (theta), typically spanning from $0$ to $2\pi$.
    * The Y dimension is mapped to the Radius (r), extending from the center to the outer edge.
* Angular Customization: Users can define the `start_angle` (e.g., starting from the 12 o'clock position) and the total `end_angle` for partial circular plots.
* Donut Configurations: By adjusting the `inner_radius`, the system can create a hole in the center, transforming a standard radial plot into a donut-style visualization.

## The Coordinate Pipeline

Regardless of the system chosen, Charton follows a unified interface for rendering:

1. Normalization: Data is first converted to a $[0, 1]$ range by the Scales.
2. Transformation: The Coordinate System takes these ratios. For example, in a Polar system, it calculates $x = r \cdot \cos(\theta)$ and $y = r \cdot \sin(\theta)$.
3. Canvas Mapping: The resulting points are scaled and shifted to fit within the `PanelContext`—the actual physical rectangle reserved for the plot.

## 8.5 Coordinate-Driven Layouts

Different coordinate systems require different aesthetic defaults. As specified in the layout logic:
* Cartesian Bars often have gaps between groups to distinguish categories.
* Polar Sectors (like in a Pie chart) often have zero spacing by default to maintain a solid, continuous circular shape.

The Coordinate System communicates these "Layout Hints" to the Marks, ensuring that the visualization looks correct out-of-the-box, whether it's a grid-based scatter plot or a radial wind rose.

---

### Key Takeaways
* Cartesian is for rectangular grids; Polar is for circular/radial views.
* Coordinate Flipping allows for easy orientation changes (Horizontal vs. Vertical).
* Coordinate Systems are the final step in the transformation pipeline, converting mathematical ratios into physical geometry.