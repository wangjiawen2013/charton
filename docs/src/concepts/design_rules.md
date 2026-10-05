# Design Rules: Marks, Tiers & Statistical Atoms

This page is the **constitution** of Charton's grammar: the definitions and rules
we agreed on, written down so they are not lost. When a new chart is proposed,
this is what decides how it is built.

It builds on [The Layer Pipeline](grammar_pipeline.md) and
[The Charton Mental Model](mental_model.md); read those first if the
stat/position/geometry split is new.

## 1. The four stages (recap)

Every layer answers four questions, in order:

| Question | Stage | Lives in |
|---|---|---|
| What values are there? | **stat** | `transform` |
| Where does each mark go? | **position** | `position` |
| What shape is drawn? | **geometry** | `mark` + `render` |
| How does a value become a pixel? | **scale** + **coordinate** | `scale`, `coordinate` |

Keeping the stages apart is what lets a handful of parts describe almost any
chart. Every rule below protects that separation.

## 2. What counts as a Mark

A **mark** is a *configuration type* implementing the [`Mark`] trait. It:

1. identifies itself with a stable string (`"point"`, `"violin"`, …);
2. carries the visual defaults (fill, stroke, opacity, …);
3. is paired with a `MarkRenderer` that turns **already-placed coordinates** into
   backend primitives.

A mark never runs statistics and never touches scales. It is a *template for
drawing*, nothing more. The renderer only ever emits circles, rectangles, lines
and paths — a backend never sees the word "violin" or "box plot".

## 3. The statistic contract (统计规范)

A **statistic** is a transform with the shape `Dataset → Dataset`. The rules:

- **It replaces the table.** Downstream `encode` must refer to the columns the
  transform *emits*, not the originals.
- **It is explicit about columns.** Each transform documents what it reads and
  what it writes. Output names are *defaults*, renameable with `with_as(...)`.
- **It runs per group and per panel.** The discrete aesthetics (colour, facet)
  define the groups; a faceted chart re-runs the statistic on each panel's own
  rows. This is why a stat-mark keeps a pre-statistic snapshot.
- **Missing values follow the ggplot2 / Vega-Lite rule.** A null in a positional
  channel drops the row; a null in a non-positional channel (colour, a transform
  group) is kept as a reserved `NA` level. See
  [Missing Values & Gaps](missing_values.md).
- **Output names must be distinct.** `ensure_distinct_columns` turns a silent
  overwrite into an error.

Statistics are **bounded**: the canonical ones are count/binning, quantiles,
confidence intervals, cumulative aggregation (a running total), 1-D KDE, 2-D KDE
and marching-squares contours. That finiteness is what bounds the number of
marks (see §6).

## 4. The geometry rules (几何规则)

- Geometry consumes placed coordinates and emits backend primitives. It knows
  nothing about statistics.
- **One geometry per shape**, with `stat = identity` unless a composite mark
  supplies a statistic (§7).
- The four "connect the dots" geometries differ by exactly two flags:
  *order* (sorted vs row order), *grouping* (`color` vs `path_group`), and
  *closed/filled*:

  | Geometry | Order | Grouped by | Closed? | Filled? |
  |---|---|---|---|---|
  | `mark_line` | sorted by x | colour | no | no |
  | `mark_path` | row order | `path_group` | no | no |
  | `mark_polygon` / `mark_geoshape` | row order | `path_group` | yes | yes |

## 5. The position rules (车道规则)

- Side-by-side layout is owned by [`Position`], in **data space**, measured in
  category steps. No renderer re-implements it.
- The dodge *arithmetic* lives in exactly one place: [`Position::offset`].
  Every renderer (bar, box, error bar, point) and every transform
  (`build_lane_layout`, used by band and quantile boxes) calls it.
- The lane *assignment* is shared too: every transform orders groups with
  `labels_with_missing` (missing last), so `sub_idx` / `groups_count` mean the
  same thing in all of them.
- The default **gap** is a deliberate per-family choice, not a global constant:
  fill/interval marks tile (`mark_bar`, `mark_errorbar` → `0.0`), while
  outline/point marks keep a gap (`mark_boxplot`, `mark_point`, and the
  band/quantile boxes → [`Position::DEFAULT_DODGE_SPACING`], `0.2`). Marks that
  are layered share a family, so bar↔error-bar and violin↔box always coincide;
  `tests/test_lane_alignment.rs` locks both pairs.
- A discrete position scale's `normalize` is affine in the category index, which
  is why "category index + offset" (the transform path) and "normalized category
  + offset·step" (the renderer path) agree exactly.

## 6. The three tiers

Chart types are **infinite**; canonical statistics are **finite**. We keep the
core mark list bounded by grouping on the *statistic × geometry* grid, not on
chart types.

### Tier 1 — geometry primitives

`stat = identity`: a row maps to a shape, and the row count is preserved.
Position logic (stacking, dodging, jitter) is allowed; statistics are not.

> `mark_point`, `mark_line`, `mark_path`, `mark_polygon` / `mark_geoshape`,
> `mark_bar`, `mark_area`, `mark_rect`, `mark_rule`, `mark_text`, `mark_tick`

`mark_rect` is the heatmap primitive; it can bin/aggregate a value, which makes
it the geometry half of the binning statistics below.

### Tier 2 — statistical atoms (统计原子)

A **statistical atom** is one canonical statistic bound to one geometry. This is
the smallest *complete* meaning of a mark, e.g. `violin = 1-D KDE × polygon`.

| Atom | Statistic × geometry |
|---|---|
| `mark_hist` | bin / count × rect |
| `mark_boxplot` | five-number summary × box + whisker + point |
| `mark_errorbar` | confidence interval × line |
| `mark_violin` | 1-D KDE × polygon |
| `mark_density` | 1-D KDE × area |
| `mark_contour` | marching squares × path |
| `mark_density_2d` | 2-D KDE × rect |

There is **one Tier 2 mark per canonical statistic** (a statistic may have more
than one geometry, e.g. KDE → density and violin). The list is closed.

`mark_boxplot` is the documented **exception** to the thin-recipe rule (§7): its
geometry is genuinely mixed (rect + line + circle) and it needs whiskers and
Tukey outliers, which the composable `transform_quantile_box` does not emit. It
therefore keeps a dedicated renderer — but it still shares `Position` for its
lanes.

### Tier 3 — compositions (recipes)

A composition stacks several marks into a picture. These are **never** core
marks; they live as examples/recipes.

> raincloud (violin + quantile box + points), ridge (one-sided KDE band +
> flip), streamgraph (area + `stack: center`), lollipop/dumbbell/range (rule +
> point), slope/bump (line + rank), candlestick (rule + floating bar),
> nightingale/rose (bar + polar), waterfall (bar + cumulative),
> marimekko/bullet, …
>
> A composition may lean on **general** options on the shared parts — a floating
> `mark_bar` (`y2`), a reversed axis (`with_reverse(true)`), a new window
> statistic (`CumulativeSum`) — as long as those options are useful beyond the
> one chart they were added for.

### The firewall: a mark must change a *cell*

The mark list stays finite because a Tier 2 mark has to occupy a **new cell** of
the `(canonical statistic × Tier-1 geometry)` grid. A new *chart name* that
reuses a cell is a Tier 3 composition, no matter how common it is:

| Name | stat × geometry | Verdict |
|---|---|---|
| Density | 1-D KDE × area | Tier 2 — `mark_density` |
| Violin | 1-D KDE × polygon | Tier 2 — `mark_violin` |
| Ridgeline | 1-D KDE × polygon | **Tier 3** — the violin's cell; only the side, overlap and orientation change |
| Histogram | bin / count × rect | Tier 2 — `mark_hist` |
| Contour | marching squares × path | Tier 2 — `mark_contour` |
| Raincloud | KDE + quantile + point | **Tier 3** — a stack of marks |
| Waterfall | bar + cumulative window | **Tier 3** — a stack of marks |

A ridge is the test case. It introduces no statistic and no geometry — it is a
KDE polygon with one bank, an overlap and a flip — so it is a recipe. What it
*does* need belongs to general options on the shared parts: `with_side`
(one-sided band) and `with_overlap` (spill past the lane) on `transform_band`.
A capability that only earns its keep for one chart is a smell; a capability
that several charts share is a grammar feature.

This is also the rule for promotion: a `mark_*` may exist as a *thin recipe*
over those options, but only when it names a cell that no other mark names.
Otherwise it is a composition, and it lives in the Cookbook.

## 7. Promotion checklist: does a chart become a Tier 2 mark?

A candidate is promoted only if **all** hold:

1. **Recurring** — it appears constantly in real analysis.
2. **Canonical recipe** — geometry and statistic have one agreed default.
3. **Not a parameter** — it cannot be expressed by tuning an existing mark
   (a *split* violin is a parameter of `mark_violin`, not a new mark).
4. **One mark, not a stack** — a raincloud is several marks, so it stays Tier 3.
5. **Clear data contract** — you can say exactly which columns it reads/writes.

A useful heuristic: a name that is a **geometry/statistic atom** (violin,
contour, density) qualifies; a name that is a **metaphor for a whole picture**
(raincloud, waterfall, bullet) is a recipe.

## 8. The thin-recipe rule

A Tier 2 composite mark must be a *thin recipe*, not a second implementation:

- it expands into **public transforms + a shared renderer** — no private
  statistic, no bespoke drawing path;
- it is **byte-for-byte identical** to the hand-written composition, locked by a
  test;
- its input columns come from the **encodings** (or an explicit argument when no
  channel exists, e.g. contour's `z`);
- it caches its resolved input columns on the mark so a **faceted re-run** can
  find them after the encoding has been rewritten to generated columns;
- its parameters are exposed through a `configure_<mark>(…)` closure;
- it carries the **original column names** into the axis/legend labels.

This is what makes the mark a *name for a combination* rather than a new thing
to learn. A component that can only live as a private branch inside
`match mark_type` is a smell.

## 9. Naming rules

- `mark_*` for marks, `transform_*` for transforms, `configure_*` for tuning.
- Keep one vocabulary. Do **not** introduce `geom_*` alongside `mark_*`.
- Generated columns are ordinary names (`x`, `y`, `path_group`, `density`,
  `level`); renaming is done with `with_as`.

## 10. Title / label rules

Axis and legend titles resolve in this order:

1. an explicit chart label (`with_x_label`, `with_color_label`, …);
2. the channel label (`alt::y("…").with_label("Original name")`);
3. the data field name.

A composite mark sets (2) so that the original column keeps naming the channel
after the encoding was rewritten to generated columns. This works for `X`, `Y`,
`Color`, `Shape` and `Size`.

## 11. Testing rules

- Every Tier 2 thin-recipe mark has a **byte-for-byte equivalence test** against
  the hand-written recipe (`tests/test_violin_mark.rs`, `test_density_mark.rs`,
  `test_contour_mark.rs`, `test_density_2d_mark.rs`).
- Lane placement is locked across marks by `tests/test_lane_alignment.rs`.
- A behavioural change to a mark should first capture a golden output, then be
  verified identical.

## 12. Current inventory

| Tier | Marks |
|---|---|
| 1 | `point`, `line`, `path`, `polygon`, `geoshape`, `bar`, `area`, `rect`, `rule`, `text`, `tick` |
| 2 | `hist`, `boxplot`, `errorbar`, `violin`, `density`, `contour`, `density_2d` |
| 3 | raincloud, ridge, streamgraph, lollipop, dumbbell, range, slope, bump, candlestick, waterfall, nightingale, rose, bullet, marimekko, … |

## 13. Glossary

| Term (used here) | Meaning |
|---|---|
| **mark** | a configuration type + renderer that draws placed coordinates |
| **statistic (统计规范)** | a `Dataset → Dataset` transform that summarises rows |
| **geometry (几何规则)** | the shape emitted from placed coordinates |
| **position (车道规则)** | the data-space side-by-side layout, owned by `Position` |
| **statistical atom (统计原子)** | a Tier 2 `statistic × geometry` pair |
| **composite mark** | a Tier 2 mark that expands into public parts |
| **thin recipe** | the rule that a composite mark must expand into public parts |
| **Tier 1 / 2 / 3** | geometry primitive / statistical atom / composition |

[`Mark`]: ../grammar/marks.md
[`Position`]: ../grammar/encodings.md
[`Position::offset`]: ../concepts/grammar_pipeline.md
