# Chapter 12 · Layout
Layout in Charton is more than just positioning elements; it is a mathematical negotiation between the **shape of your data** and the **geometry of the coordinate system**. While many libraries rely on hard-coded logic to switch between grouped and single-column charts, Charton uses a unified strategy to handle all positioning.

## 12.1. The Row-Stub Layout Strategy
The **Row-Stub Layout Strategy** is the core engine behind how Charton decides the width, spacing, and offset of marks (bars, sectors, or boxes). Instead of asking "Is this a grouped chart?", the renderer asks: "**How many rows of data exist for this specific coordinate?**"

### 12.1.1. The Philosophy: Data Shapes Geometry
In the Row-Stub model, the "physical" presence of a data row in the transformed DataFrame acts as a "stub" or a placeholder in the visual layout.
* **Single Row per Category**: If a category (e.g., "Category A" on the X-axis) contains exactly one row, the mark occupies the **full available span**.
* **Multiple Rows per Category**: If a category contains $N$ rows, the layout engine automatically carves the available space into $N$ sub-slots.

### 12.1.2. The Mechanism: Cartesian Normalization
To ensure consistent layouts, Charton performs a **Cartesian Product** during the data transformation phase. If you encode both `x` and `color`, Charton ensures that every unique value of `x` has a row for every unique value of `color`.

1. **Gap Filling**: If "Category A" has data for "Male" but not "Female," Charton inserts a "Female" row with a value of `0`.
2. **Stable Count**: This ensures that every X-axis slot has the exact same number of rows ($N$).
3. **Implicit Positioning**: he renderer simply iterates through these $N$ rows. The $i$-th row is automatically placed at the $i$-th sub-slot.

### 12.1.3. Dimension Deduplication: Intent Recognition
The layout engine must first distinguish between **visual aesthetics** (just adding color) and **structural dimensions** (splitting data into groups). Charton achieves this through **Automatic Column Deduplication** during the preprocessing phase.

Before the Row-Stub engine calculates $N$ (the number of rows per slot), it performs the following check:

1. **The Dimension Set**: Charton collects all fields used in `x`, `color`, `size`, and `shape`.
2. **Deduplication**: If a field is used in both a positional channel (like `x`) and a styling channel (like `color`), it is only counted **once** as a grouping dimension.
3. **Intent Recognition**:
- `x("type"), color("type")`: After deduplication, there is only **one** grouping dimension (`type`). The engine recognizes this as a **Self-Mapping** intent—use colors to distinguish categories, but keep them in a single, full-width slot.
- `x("type"), color("gender")`: There are **two** distinct dimensions. The engine recognizes this as a **Grouping** intent—sub-divide each `type` slot by `gender`.

Without this deduplication step, a Rose Chart would mistakenly try to "dodge" (place side-by-side) the same category against itself, leading to overlapping marks or unnecessarily thin sectors.

### 12.1.4. The Mechanism: Cartesian Normalization
To ensure consistent layouts across all categories, Charton performs a **Cartesian Product** based on the *deduplicated* dimension set.

1. **Grid Creation**: If `x` has 5 unique values and `color` (a different field) has 2, Charton creates a "Layout Grid" of $5 \times 2 = 10$ rows.
2. **Gap Filling**: If "Category A" has data for "Male" but not "Female," Charton joins the grid with the raw data and inserts a "Female" row with a value of `0`.
3. **Physical Alignment**: This ensures that every X-axis slot has the exact same number of physical row stubs ($N=2$).
4. **Predictable Offsets**: The renderer simply iterates through these $N$ rows. The $i$-th row is always placed at the $i$-th sub-slot, ensuring that "Male" is always the left bar and "Female" is always the right bar, even if the data for one is missing.

### 12.1.5. Mathematical Resolution
The physical width of a mark in normalized space is calculated using the following derived formula:
$$\text{Mark Width} = \frac{\text{Slot Span}}{N + (N - 1) \times \text{Spacing}}$$
Where:
- **$N$**: The number of deduplicated row stubs for that category.
- **Slot Span**: The total percentage of the category width used (default 0.7-1.0).
- **Spacing**: The inner padding between bars within the same group.

### 12.1.6. Advantages of Row-Stub Layout
1. **Consistency**: Bars never "jump" positions if data is missing; the "0" value row keeps the slot occupied.
2. **Polar-Cartesian Parity**: The same logic that creates side-by-side bars in Cartesian coordinates creates perfectly partitioned sectors in a Rose Chart.
3. **Zero Hard-coding**: The renderer doesn't need to know if the chart is "Grouped" or "Stacked"—it simply follows the rows provided by the deduplicated data engine.

## 12.2. The Legend Layout Strategy
## 12.3 The Axis Layout Strategy
