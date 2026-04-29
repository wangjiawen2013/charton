# Semaglutide Weight Loss Curve (NEJM 2021)

## Background
This figure is a reproduction of **Figure 1A** from the landmark study *"Once-Weekly Semaglutide in Adults with Overweight or Obesity"*, published in **The New England Journal of Medicine (NEJM)** in 2021. The study evaluates the efficacy and safety of semaglutide as a pharmacological intervention for weight management.

The plot illustrates the **mean percentage change in body weight** over a 68-week period. It highlights the significant divergence in weight loss trajectories between the semaglutide group and the placebo group, both of which were conducted alongside lifestyle interventions.

## Data Acquisition
The data used for this visualization was extracted from the original publication using [WebPlotDigitizer](https://automeris.io/).

## Implementation
Using Charton’s "Grammar of Graphics" approach, we can recreate this complex clinical plot by layering multiple graphical components, enabling highly flexible and customizable visualizations with concise Rust code.

```rust
{{#include ../../../examples/weight_loss_curve.rs}}
```

<img src="../images/weight_loss_curve.svg" width="500">