# Writing a chart document

A chart can be written as a JSON document instead of as code. The document names
its data, the mark to draw and which column goes where:

```json
{
  "$schema": "https://charton.dev/schema/charton-v0.schema.json",
  "data": { "url": "sales.csv" },
  "mark": "bar",
  "encoding": {
    "x": { "field": "region" },
    "y": { "field": "revenue", "aggregate": "sum" },
    "color": { "field": "segment" }
  },
  "title": "Revenue by region"
}
```

Render it with `charton render chart.json -o report.pdf`. The CLI, the Nushell
plugin, and the MCP server all use this document format.

Documents exist for three reasons: they can be **saved** next to the data they
describe, **reviewed** like any other source file, and **generated** — by a
script, by a colleague, or by an agent — with a validator that says exactly what
is wrong before anything is drawn.

## What the parts mean

| Key | What it says |
|---|---|
| `data` | Where the rows come from: a local path, or inline rows. Only the root has data; layers inherit it. |
| `mark` | What to draw, in Vega-Lite's vocabulary. Charton-only marks (`violin`, `contour`, …) go under `charton.mark` instead. |
| `encoding` | Which column feeds which channel: `x`, `y`, `y2`, `color`, `size`, `shape`, `text`, `pathGroup`. |
| `transform` | Row operations, applied in order before the mark: `filter`, `fold`, `aggregate`, `bin`, `window`. |
| `layer` | Several marks on one set of axes. |
| `facet` | Split the chart into panels by one field, or by two. |
| `title`, `description`, `theme` | Presentation and accessibility. |

Every field is checked. An unsupported key is an error, never a silent no-op.
The full per-field reference — what each field becomes in the engine, and where
it is refused — is [the field reference](lowering-map.md).

## How a document is processed

```text
read the data ──► apply `transform` ──► validate ──► build ──► render
```

Validation runs twice, in two tiers:

1. **The document alone** — is the shape right, is the mark one Charton knows,
   does the encoding fit that mark, is the facet spelled one way. Structural
   problems are reported one at a time, because nothing after a malformed field
   can be trusted. Everything else is reported all at once.
2. **With the data** — do the fields exist, can their values serve the channel,
   would the mark's own statistics draw something the document did not say. This
   tier runs only when rows are available.

Both tiers happen before a chart is built. That is why every diagnostic can
point at the exact place in the document that needs changing.

## When it is wrong

`charton validate chart.json --data sales.csv --json` prints:

```json
{
  "ok": false,
  "errors": [
    {
      "code": "E_UNKNOWN_MARK",
      "at": "/mark",
      "message": "Unknown mark `linee`",
      "hint": "Did you mean `line`?",
      "expected": ["point", "line", "bar", "area"]
    }
  ]
}
```

- `code` is stable; scripts and agents branch on it.
- `at` is a JSON Pointer into this document.
- `hint` and `expected` say what to try next.

The families mean different things on purpose. `E_UNKNOWN_*` means the word is
wrong and the candidates are listed. `E_UNSUPPORTED_*` means the word is valid
Vega-Lite that Charton does not implement. `E_DATA_*` means the document is fine
and the data is not. See the [command line guide](../getting_started/cli.md) for
the exit codes.

## Versioning

The `$schema` string is the only version carrier, and it is Charton's own URL —
not Vega-Lite's. Pointing at the Vega-Lite schema would make an editor accept the
whole language and then fail here.

Charton implements a **subset** of Vega-Lite v5 and says so. Fields outside the
subset are refused, so a document that validates is a document that renders. The
spec version (`v0`) moves independently of the crate version.

## What a document is not

It is data, not a program. There are no expression strings, no `calculate`, no
closures, and no file or network access from a document. When a chart needs a
derived value, compute it upstream and put the column in the data.

## Elsewhere

- [Field reference](lowering-map.md) — every field, and what it becomes.
- [The command line](../getting_started/cli.md) — `plot`, `render`, `validate`,
  `schema`, and `--emit-spec`.
- [Agents (MCP)](../getting_started/mcp.md) — the same checks, served as tools.
- [The Nushell plugin](../ecosystem/nushell.md#chart-documents) — `charton render`
  draws the same document inside a pipeline.
