### Precision-Preserving `i128` Math
Mapping a 19-digit nanosecond timestamp directly to a screen pixel via `f32` or even `f64` introduces Bit-Drift. At large Unix offsets (e.g., year 2026), `f64` lacks the sufficient mantissa bits to distinguish between nanosecond intervals, resulting in a "staircase effect" or jittery rendering during high-zoom interactions.

Charton solves this by performing all internal domain calculations in `i128` space before converting to a normalized ratio.

### Relative Anchoring & Jitter Prevention
To maintain sub-pixel accuracy, the engine utilizes a Local Anchor Strategy. Instead of calculating global coordinates, it calculates positions relative to the current view's minimum value ($T_{anchor}$):

|Step|Operation|Logic|
|----|---------|-----|
|1. Anchoring|$Toffset​=(Tcurrent​−Tanchor​)$|Integer subtraction in `i128`|
|2. Normalizing|$Ratio=Toffset(f64)​/Range(f64)$​|Safe float division|
|3. Projecting|$Pixel=Ratio×ViewportSize$​|Map to $f32$ for GPU|

By subtracting the large epoch offset in the integer domain first, the resulting delta is small enough to be represented with perfect fidelity in an `f64` ratio, ensuring a smooth rendering experience even at microsecond-level zoom.

### Automatic Scale Degradation (Cosmic Scales)
When the temporal span exceeds the `i64` limit or the capabilities of standard calendar libraries (e.g., spans of millions of years), the Temporal Engine undergoes Semantic Degradation:

* Calendar Mode: Active for spans fitting within the `i64` range. Supports leap years, months, and weekdays via the `time` crate.
* Numerical Mode (Deep Time): Active for cosmic or geological scales. The engine stops attempting to format "Tuesdays" or "Months" and treats the input as a raw numerical axis (e.g., "13.8 Billion Years"), using scientific notation or custom unit-based labels.