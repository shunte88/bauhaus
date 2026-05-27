# Bauhaus Pattern Generator

A small, fast Rust CLI that generates Bauhaus-style SVG patterns by tiling random shape components onto a grid with a user-supplied colour palette, theme, and style.

## Build

```
cargo build --release
```

The binary lands at `target/release/bauhaus`.

## Usage

```
bauhaus --config <path.yaml> [--assets <dir>] [--output <path>]
bauhaus --theme  <path.svg> [--config <path.yaml>] [--assets <dir>] [--output <path>]
bauhaus --list-palettes
```

`--config` or `--theme` is required (or `--list-palettes`). When both are passed, config overrides theme settings (except grid dimensions, those always come from the theme).

Output defaults to `bauhaus_YYYYMMDD_HHMMSS.svg`.

## Config

```yaml
palette: "Modern Noir"   # named palette OR inline array of 1..=10 hex strings
background: "#000000"    # optional; auto-picked when palette is named
columns: 12              # 4..=20
rows: 8                  # 4..=20
size: 40                 # 30..=60 (cell side in px)

method: spiral           # grid (default) | wave | spiral | inflection | deflection
supersize: true          # accent glyphs scaled 3×/4×/6× (10..=24 of them)
lines: true              # bold lines crossing the canvas (4..=12 of them)
limit: 8                 # cap working glyphs to 8 randomly-selected ones
```

All keys above `method` are required in standalone mode. Everything from `method` down is optional.

## Feature behaviour

| Feature | What it does |
| --- | --- |
| `method: grid` | Random component, random rotation, random colour per cell. The chaotic baseline. |
| `method: wave` | Phase = `sin(col·freq + row·0.3)`. Sinusoidal colour bands across the canvas. |
| `method: spiral` | Phase = `(angle/2π + 2·r/max_r) mod 1`. Colour arms wound from the centre. |
| `method: inflection` | Phase = `½ + ½·tanh((col−cx)/cx · 2.5)`. S-curve transition L↔R. |
| `method: deflection` | Phase = `\|col−cx\|/cx`. V-shape symmetric about the centre column. |
| `supersize: true` | 10–24 extra components placed at scale 3×, 4×, or 6× as accent pieces. Can roll off any edge. |
| `lines: true` | 4–12 bold lines (horizontal, vertical, or 45° in either direction). Length `[size, max-for-orientation]`, stroke-width `[2, size/2]`. Drawn on top of everything. |
| `limit: N` | Randomly pick N glyphs from the loaded asset set and use only those for the whole pattern. `N ≥ 1`; clamped silently if `N` exceeds available glyphs. |

For non-`grid` methods, colour selection inside each cell is biased: ~65% the dominant `palette[phase·N]`, ~35% an immediate neighbour. Rotation is quantised to `{0°, 90°, 180°, 270°}` indexed by phase, so neighbouring cells in the same phase region orient together.

## Theme mode

`--theme <path.svg>` reseeds from a previously-generated bauhaus SVG. Every output embeds a `<desc>bauhaus...</desc>` metadata block that round-trips cleanly.

| Invocation | Grid (cols/rows/size) | Palette / bg | supersize / lines / limit / method |
| --- | --- | --- | --- |
| `--config X` | from config | from config | from config |
| `--theme T` | from theme | from theme | from theme |
| `--theme T --config X` | **theme** (config grid keys ignored, with stderr note) | config overrides if present | each key overrides if present |

A theme reseed with `limit=N` picks a *fresh* random N-subset of glyphs, the count is preserved, but not the specific glyphs [TBD add consistent keyword].

## Built-in palettes

Case- and punctuation-insensitive lookup. List all with `bauhaus --list-palettes`.

```
Modern Noir       Exhibition 1923   Primary           Kandinsky
Sunset            Ocean             Albers            Moholy-Nagy
Spring            Summer            Autumn (aka Fall) Winter
Monochrome (aka Grayscale)          Vibrant
Comiskey01 (aka barrett01)          Comiskey02 (aka barrett02)
```

Each named palette has a foreground colour set plus a background pool; the background is auto-picked when you reference a palette by name. Use `background: "#..."` to override, or use an inline `palette: [...]` array to skip the background entirely.

You can also specify your own palette with up to 10 supported colours.

## Assets

Shapes are SVG files in the `--assets` directory (default `assets/`). Each file uses a `viewBox="0 0 100 100"` base coordinate space and marks paintable fills with the literal placeholder `{{C}}` : every occurrence is independently replaced with a chosen palette colour at generation time. Drop a new SVG in the folder and it's picked up on the next run.

Bundled sets:

- `assets/` : the default 31 primitives (squares, circles, triangles, diamonds, arches, half/quarter circles, hourglass, triangle-grid, lines, cross, pill, ring, spiral, diagonal stripes, dot rows, splits, quadrants, dots, semis, etc.).
- `asset_set_01/` : 42 folk-style ornaments (quatrefoils, snowflakes, stars, flowers, scrollwork, hearts, sunbursts).
- `asset_set_02/` : 18 large composite tiles (tulips, lotus, bullseye, half-disks, leaf stems, dot rows, etc.).

Shapes and palettes are derived from exemplar patterns provided by Barrett/Peter for their e-Ink project, multiple reference sourced from pinterest, as well as hand-drawn additions.
