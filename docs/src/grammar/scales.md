# Scales & Domains

If Encoding defines *which* data fields are connected to which visual channels, then the Scale defines *how* that connection is mathematically calculated. A Scale is a function that maps values from a Data Domain to a Visual Range.

## Core Concepts: Domain and Range

Every Scale operates between two distinct spaces:

1. Domain: The state of the data. For example, a temperature range in your dataset $[0^\circ\text{C}, 100^\circ\text{C}]$ or a set of categories like `["Apple", "Banana", "Orange"]`.
2. Range: The state of the visual output. For an axis, this is typically physical pixels (e.g., $[0, 800]$); for a color encoding, it is a sequence of colors in a palette.

## Scale Types

Charton provides specialized scale types based on the nature of the underlying data (Quantitative, Categorical, or Temporal):

### Linear Scale

The most common scale for quantitative data. It preserves the original proportional relationships between data points using a linear transformation ($y = ax + b$).

* Best for: Prices, lengths, weights, and counts.
* Expansion: To prevent data marks (like scatter points) from being cut off at the edges, Charton automatically applies a $5\%$ padding to both ends of the Domain by default.

### Logarithmic (Log) Scale

When data spans multiple orders of magnitude (e.g., $1$ to $1,000,000$), a linear scale compresses small values. A Log Scale applies a logarithmic transformation, ensuring that each order of magnitude occupies an equal visual weight.

* Note: The Domain of a Log Scale cannot contain zero or negative numbers.

### Discrete (Ordinal) Scale

Used for categorical or ranked data. Unlike continuous scales, it partitions the visual range into discrete "slots" or "bins."

* Best for: Country names, product categories, or ratings (e.g., Poor/Fair/Good).
* Stability: The system uses stable sorting for categories to ensure that the order of items remains consistent across multiple renders.

### Temporal Scale

Specifically designed for dates and times. It understands the irregular spans of days, hours, and minutes, and automatically generates human-readable axis ticks (e.g., showing months instead of raw timestamps).

## Ticks and Label Generation

Scales do more than just map positions; they communicate meaning through Guides.

- Tick Generation: The system calculates "pretty" numbers for axis marks. If your data range is $[0, 93]$, the scale will intelligently choose $[0, 20, 40, 60, 80, 100]$ as ticks rather than arbitrary values.
- Formatting: Support for scientific notation, currency symbols, percentages, and custom date-time formats.

## Scale Arbitration

In a multi-layered chart, Scales act as a "Single Source of Truth." When different layers share an axis, Charton performs Arbitration:

1. Scanning: The engine identifies the "Global Union" of all data domains across all layers.
2. Unification: A single, unified scale is created that is large enough to encompass the data from every layer.
3. Distribution: This unified scale is injected back into each layer, ensuring that they are all drawn within the same mathematical coordinate system.

## Manual Overrides

While Charton automates most scaling logic, you retain full control through explicit overrides:

- Force Zero: Ensure an axis starts at $0$ even if the minimum data point is much higher.
- Fixed Intervals: Fix a percentage axis strictly between $[0, 1]$ to prevent it from auto-scaling based on a subset of data.

## Key Takeaways

- Domain is data; Range is pixels or colors.
- Scale Type determines the "visual density" (Linear vs. Log).
- Automatic Arbitration ensures that composite layers are perfectly aligned.
- The Tick System translates abstract math into readable information.