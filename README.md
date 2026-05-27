# bauhaus

A small, fast Rust CLI that generates Bauhaus-style SVG patterns by tiling random shape components onto a grid with a user-supplied colour palette.

## Build

```
cargo build --release
```

The binary lands at `target/release/bauhaus`.

## Usage

```
bauhaus --config <path.yaml> [--assets <dir>] [--output <path>]
bauhaus --list-palettes
```

## Config

```yaml
palette: "Modern Noir"   # or an inline array of 1..=10 hex strings
background: "#000000"    # optional; auto-picked when palette is named
columns: 12              # 4..=20
rows: 8                  # 4..=20
size: 40                 # 20..=50 (cell side in px)
```

Built-in palettes (case- and punctuation-insensitive): `Modern Noir`, `Exhibition 1923`, `Primary`, `Kandinsky`, `Sunset`, `Ocean`, `Albers`, `Moholy-Nagy`. Shapes and palettes are ports of [bauhaus.graphics](https://www.bauhaus.graphics/).

Output defaults to `bauhaus_YYYYMMDD_HHMMSS.svg`.
