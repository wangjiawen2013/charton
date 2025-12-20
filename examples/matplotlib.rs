use charton::prelude::*;
use polars::prelude::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let exe_path = r"D:\Programs\miniconda3\envs\cellpy\python.exe";
    let df = df![
        "a" => [1],
        "b" => [2],
    ]?; // A toy dataset

    let raw_plotting_code = r#"
import matplotlib.pyplot as plt
import numpy as np
import matplotlib.tri as tri

# First create the x and y coordinates of the points.
n_angles = 36
n_radii = 8
min_radius = 0.25
radii = np.linspace(min_radius, 0.95, n_radii)

angles = np.linspace(0, 2 * np.pi, n_angles, endpoint=False)
angles = np.repeat(angles[..., np.newaxis], n_radii, axis=1)
angles[:, 1::2] += np.pi / n_angles

x = (radii * np.cos(angles)).flatten()
y = (radii * np.sin(angles)).flatten()
z = (np.cos(radii) * np.cos(3 * angles)).flatten()

# Create the Triangulation; no triangles so Delaunay triangulation created.
triang = tri.Triangulation(x, y)

# Mask off unwanted triangles.
triang.set_mask(np.hypot(x[triang.triangles].mean(axis=1),
                         y[triang.triangles].mean(axis=1))
                < min_radius)
fig1, ax1 = plt.subplots(figsize=(4.2, 3.2))
ax1.set_aspect('equal')
tpc = ax1.tripcolor(triang, z, shading='flat')
fig1.colorbar(tpc)
ax1.set_title('tripcolor of Delaunay triangulation, flat shading')
    "#;

    Plot::<Matplotlib>::build(data!(&df)?)?
        .with_exe_path(exe_path)?
        .with_plotting_code(raw_plotting_code)
        .save("./examples/matplotlib.png")?;

    Ok(())
}
