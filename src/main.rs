mod palettes;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chrono::Local;
use clap::Parser;
use rand::Rng;
use rand::seq::SliceRandom;
use serde::Deserialize;

const COLOR_TOKEN: &str = "{{C}}";
const COMPONENT_UNIT: u32 = 100;
const ROTATIONS: [u32; 4] = [0, 90, 180, 270];
const SUPERSIZE_SCALES: [u32; 3] = [3, 4, 6];
const SUPERSIZE_COUNT_RANGE: std::ops::RangeInclusive<u32> = 10..=24;
const LINES_COUNT_RANGE: std::ops::RangeInclusive<u32> = 4..=12;
const META_OPEN: &str = "<desc>bauhaus";
const META_CLOSE: &str = "</desc>";

#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Method {
    #[default]
    Grid,
    Wave,
    Spiral,
    Inflection,
    Deflection,
}

impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Method::Grid => "grid",
            Method::Wave => "wave",
            Method::Spiral => "spiral",
            Method::Inflection => "inflection",
            Method::Deflection => "deflection",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "grid" => Ok(Method::Grid),
            "wave" => Ok(Method::Wave),
            "spiral" => Ok(Method::Spiral),
            "inflection" => Ok(Method::Inflection),
            "deflection" => Ok(Method::Deflection),
            other => bail!(
                "unknown method {other:?}; expected one of: grid, wave, spiral, inflection, deflection"
            ),
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "bauhaus", about = "Generate Bauhaus-style SVG patterns")]
struct Args {
    /// YAML config. Required unless --theme or --list-palettes is given.
    #[arg(short, long, required_unless_present_any = ["theme", "list_palettes"])]
    config: Option<PathBuf>,

    /// A previously-generated SVG to seed grid/palette/supersize from. Config (if any)
    /// can override palette/background/supersize; grid dimensions always come from theme.
    #[arg(short, long)]
    theme: Option<PathBuf>,

    #[arg(short, long, default_value = "assets")]
    assets: PathBuf,

    #[arg(short, long)]
    output: Option<PathBuf>,

    /// List the built-in palette names and exit.
    #[arg(long)]
    list_palettes: bool,
}

#[derive(Debug)]
enum PaletteSpec {
    Named(String),
    Inline(Vec<String>),
}

impl<'de> Deserialize<'de> for PaletteSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = serde_yml::Value::deserialize(deserializer)?;
        match value {
            serde_yml::Value::String(s) => Ok(PaletteSpec::Named(s)),
            serde_yml::Value::Sequence(seq) => {
                let mut colors = Vec::with_capacity(seq.len());
                for (i, item) in seq.into_iter().enumerate() {
                    match item {
                        serde_yml::Value::String(s) => colors.push(s),
                        other => {
                            return Err(D::Error::custom(format!(
                                "palette[{i}]: expected hex string, got {other:?}"
                            )));
                        }
                    }
                }
                Ok(PaletteSpec::Inline(colors))
            }
            other => Err(D::Error::custom(format!(
                "palette: expected a built-in palette name (string) or an array of hex strings, got {other:?}"
            ))),
        }
    }
}

/// All-optional config. In standalone mode we re-validate that the required
/// fields are present; in theme mode the present fields are treated as overrides.
#[derive(Deserialize, Debug, Default)]
struct Config {
    #[serde(default)]
    palette: Option<PaletteSpec>,
    #[serde(default)]
    background: Option<String>,
    #[serde(default)]
    supersize: Option<bool>,
    #[serde(default)]
    lines: Option<bool>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    method: Option<Method>,
    #[serde(default)]
    columns: Option<u32>,
    #[serde(default)]
    rows: Option<u32>,
    #[serde(default)]
    size: Option<u32>,
}

#[derive(Debug, Clone)]
struct Resolved {
    colors: Vec<String>,
    background: Option<String>,
    supersize: bool,
    lines: bool,
    limit: Option<u32>,
    method: Method,
    columns: u32,
    rows: u32,
    size: u32,
}

fn validate_hex(c: &str) -> Result<()> {
    let s = c.trim_start_matches('#');
    if s.len() != 6 || !s.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("invalid hex color: {c:?} (expected 6 hex digits, optional '#')");
    }
    Ok(())
}

fn validate_dims(columns: u32, rows: u32, size: u32) -> Result<()> {
    if !(4..=20).contains(&columns) {
        bail!("columns must be between 4 and 20 (got {columns})");
    }
    if !(4..=20).contains(&rows) {
        bail!("rows must be between 4 and 20 (got {rows})");
    }
    if !(30..=60).contains(&size) {
        bail!("size must be between 30 and 60 (got {size})");
    }
    Ok(())
}

fn resolve_palette_spec(
    p: PaletteSpec,
    rng: &mut impl Rng,
) -> Result<(Vec<String>, Option<String>)> {
    match p {
        PaletteSpec::Named(name) => {
            let pal = palettes::lookup(&name).ok_or_else(|| {
                anyhow!(
                    "unknown palette {name:?}; available: {}",
                    palettes::names().join(", ")
                )
            })?;
            let colors: Vec<String> = pal.colors.iter().map(|s| s.to_string()).collect();
            let bg = pal.backgrounds.choose(rng).map(|s| s.to_string());
            Ok((colors, bg))
        }
        PaletteSpec::Inline(v) => {
            if v.is_empty() || v.len() > 10 {
                bail!("palette must contain between 1 and 10 colors (got {})", v.len());
            }
            for c in &v {
                validate_hex(c)?;
            }
            Ok((v, None))
        }
    }
}

/// Standalone mode: config must specify everything.
fn resolve_standalone(cfg: Config) -> Result<Resolved> {
    let mut rng = rand::thread_rng();
    let palette = cfg
        .palette
        .ok_or_else(|| anyhow!("config requires 'palette' when no --theme is given"))?;
    let columns = cfg
        .columns
        .ok_or_else(|| anyhow!("config requires 'columns' when no --theme is given"))?;
    let rows = cfg
        .rows
        .ok_or_else(|| anyhow!("config requires 'rows' when no --theme is given"))?;
    let size = cfg
        .size
        .ok_or_else(|| anyhow!("config requires 'size' when no --theme is given"))?;

    let (colors, palette_bg) = resolve_palette_spec(palette, &mut rng)?;
    if let Some(bg) = &cfg.background {
        validate_hex(bg)?;
    }
    validate_dims(columns, rows, size)?;

    if let Some(lim) = cfg.limit {
        if lim < 1 {
            bail!("limit must be >= 1 (got {lim})");
        }
    }

    Ok(Resolved {
        colors,
        background: cfg.background.or(palette_bg),
        supersize: cfg.supersize.unwrap_or(false),
        lines: cfg.lines.unwrap_or(false),
        limit: cfg.limit,
        method: cfg.method.unwrap_or_default(),
        columns,
        rows,
        size,
    })
}

/// Theme mode: take base from theme; apply present-only overrides from config.
fn apply_overrides(mut base: Resolved, cfg: Config) -> Result<Resolved> {
    let mut rng = rand::thread_rng();
    if let Some(palette) = cfg.palette {
        let (colors, palette_bg) = resolve_palette_spec(palette, &mut rng)?;
        base.colors = colors;
        if let Some(bg) = palette_bg {
            base.background = Some(bg);
        }
    }
    if let Some(bg) = cfg.background {
        validate_hex(&bg)?;
        base.background = Some(bg);
    }
    if let Some(ss) = cfg.supersize {
        base.supersize = ss;
    }
    if let Some(ln) = cfg.lines {
        base.lines = ln;
    }
    if let Some(lim) = cfg.limit {
        if lim < 1 {
            bail!("limit must be >= 1 (got {lim})");
        }
        base.limit = Some(lim);
    }
    if let Some(m) = cfg.method {
        base.method = m;
    }
    if cfg.columns.is_some() || cfg.rows.is_some() || cfg.size.is_some() {
        eprintln!("note: 'columns'/'rows'/'size' in config are ignored when --theme is used");
    }
    Ok(base)
}

fn read_theme(path: &Path) -> Result<Resolved> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading theme {}", path.display()))?;
    let open = raw.find(META_OPEN).ok_or_else(|| {
        anyhow!(
            "{} has no bauhaus metadata; only SVGs produced by this tool can be used as themes",
            path.display()
        )
    })?;
    let body_start = open + META_OPEN.len();
    let body_rel_end = raw[body_start..]
        .find(META_CLOSE)
        .ok_or_else(|| anyhow!("malformed metadata in {}: missing </desc>", path.display()))?;
    let body = &raw[body_start..body_start + body_rel_end];

    let mut columns: Option<u32> = None;
    let mut rows: Option<u32> = None;
    let mut size: Option<u32> = None;
    let mut supersize = false;
    let mut lines = false;
    let mut limit: Option<u32> = None;
    let mut method = Method::default();
    let mut colors: Vec<String> = Vec::new();
    let mut background: Option<String> = None;

    for line in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (k, v) = match line.split_once('=') {
            Some(kv) => kv,
            None => continue,
        };
        match k {
            "columns" => columns = Some(v.parse().context("metadata: columns")?),
            "rows" => rows = Some(v.parse().context("metadata: rows")?),
            "size" => size = Some(v.parse().context("metadata: size")?),
            "supersize" => supersize = v == "true",
            "lines" => lines = v == "true",
            "limit" => {
                limit = if v.is_empty() || v == "none" {
                    None
                } else {
                    Some(v.parse().context("metadata: limit")?)
                };
            }
            "method" => method = Method::parse(v).context("metadata: method")?,
            "palette" => {
                colors = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            "background" => {
                background = if v.is_empty() || v == "none" { None } else { Some(v.to_string()) };
            }
            _ => {}
        }
    }

    let columns = columns.ok_or_else(|| anyhow!("theme metadata missing 'columns'"))?;
    let rows = rows.ok_or_else(|| anyhow!("theme metadata missing 'rows'"))?;
    let size = size.ok_or_else(|| anyhow!("theme metadata missing 'size'"))?;
    if colors.is_empty() {
        bail!("theme metadata missing 'palette'");
    }
    for c in &colors {
        validate_hex(c)?;
    }
    if let Some(bg) = &background {
        validate_hex(bg)?;
    }
    validate_dims(columns, rows, size)?;

    Ok(Resolved {
        colors,
        background,
        supersize,
        lines,
        limit,
        method,
        columns,
        rows,
        size,
    })
}

fn load_components(dir: &Path) -> Result<Vec<String>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading assets dir {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("svg")))
        .collect();
    paths.sort();

    let mut components = Vec::with_capacity(paths.len());
    for path in &paths {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let inner = extract_svg_inner(&raw)
            .with_context(|| format!("parsing {}", path.display()))?;
        components.push(inner);
    }
    if components.is_empty() {
        return Err(anyhow!("no .svg components found in {}", dir.display()));
    }
    Ok(components)
}

fn extract_svg_inner(svg: &str) -> Result<String> {
    let open = svg.find("<svg").ok_or_else(|| anyhow!("missing <svg> element"))?;
    let body_start = svg[open..]
        .find('>')
        .ok_or_else(|| anyhow!("malformed <svg> tag"))?
        + open
        + 1;
    let body_end = svg.rfind("</svg>").ok_or_else(|| anyhow!("missing </svg>"))?;
    if body_end <= body_start {
        bail!("empty svg body");
    }
    Ok(svg[body_start..body_end].trim().to_string())
}

fn normalize_color(c: &str) -> String {
    if c.starts_with('#') { c.to_string() } else { format!("#{c}") }
}

/// Each method maps a cell (col, row) to a phase in [0, 1].
fn cell_phase(method: Method, col: u32, row: u32, cols: u32, rows: u32) -> f64 {
    use std::f64::consts::PI;
    let nc = col as f64;
    let nr = row as f64;
    let cx = (cols as f64 - 1.0) / 2.0;
    let cy = (rows as f64 - 1.0) / 2.0;
    match method {
        Method::Grid => 0.0, // unused; caller uses random
        Method::Wave => {
            let freq = 2.0 * PI * 1.5 / cols.max(1) as f64;
            ((nc * freq + nr * 0.3).sin() + 1.0) * 0.5
        }
        Method::Spiral => {
            let dx = nc - cx;
            let dy = nr - cy;
            let max_r = (cx.powi(2) + cy.powi(2)).sqrt().max(1.0);
            let angle = dy.atan2(dx); // [-π, π]
            let angle_norm = (angle + PI) / (2.0 * PI); // [0, 1]
            let radius_norm = (dx * dx + dy * dy).sqrt() / max_r;
            (angle_norm + radius_norm * 2.0).rem_euclid(1.0)
        }
        Method::Inflection => {
            let x = (nc - cx) / cx.max(1.0); // [-1, 1]
            0.5 + 0.5 * (x * 2.5).tanh()
        }
        Method::Deflection => {
            let x = (nc - cx).abs() / cx.max(1.0);
            x.clamp(0.0, 1.0)
        }
    }
}

/// Pick a colour from the palette weighted toward `palette[phase·N]`.
/// For Method::Grid (and 1-colour palettes) this is just a uniform random pick.
/// Other methods pick the dominant phase colour ~65% of the time and an
/// immediate neighbour the rest, so each cell reads as belonging to a clear
/// colour region.
fn pick_color<'a>(
    palette: &'a [String],
    method: Method,
    phase: f64,
    rng: &mut impl Rng,
) -> &'a str {
    let n = palette.len();
    if n == 1 || method == Method::Grid {
        return palette.choose(rng).expect("palette non-empty").as_str();
    }
    let center = (phase * n as f64).floor() as i64;
    let offset = if rng.gen_bool(0.65) {
        0
    } else if rng.gen_bool(0.5) {
        -1
    } else {
        1
    };
    let idx = (center + offset).rem_euclid(n as i64) as usize;
    palette[idx].as_str()
}

fn substitute_colors_phased(
    template: &str,
    palette: &[String],
    method: Method,
    phase: f64,
    rng: &mut impl Rng,
) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(idx) = rest.find(COLOR_TOKEN) {
        out.push_str(&rest[..idx]);
        let color = pick_color(palette, method, phase, rng);
        out.push_str(&normalize_color(color));
        rest = &rest[idx + COLOR_TOKEN.len()..];
    }
    out.push_str(rest);
    out
}

/// Rotation choice. Grid is fully random; other methods quantise to one of
/// {0, 90, 180, 270} indexed by phase, so neighbouring cells in the same phase
/// region orient together.
fn pick_rotation(method: Method, phase: f64, rng: &mut impl Rng) -> u32 {
    match method {
        Method::Grid => *ROTATIONS.choose(rng).expect("rotations non-empty"),
        _ => {
            let idx = ((phase * 4.0).floor() as usize).min(3);
            ROTATIONS[idx]
        }
    }
}

fn metadata_block(r: &Resolved) -> String {
    let normalized: Vec<String> = r.colors.iter().map(|c| normalize_color(c)).collect();
    let bg = r
        .background
        .as_deref()
        .map(normalize_color)
        .unwrap_or_else(|| "none".to_string());
    let limit_str = r
        .limit
        .map(|n| n.to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "  <desc>bauhaus\ncolumns={cols}\nrows={rows}\nsize={size}\nsupersize={ss}\nlines={lines}\nlimit={limit}\nmethod={method}\npalette={pal}\nbackground={bg}\n</desc>\n",
        cols = r.columns,
        rows = r.rows,
        size = r.size,
        ss = r.supersize,
        lines = r.lines,
        limit = limit_str,
        method = r.method.as_str(),
        pal = normalized.join(","),
        bg = bg,
    )
}

fn generate_svg(r: &Resolved, components: &[String]) -> String {
    let mut rng = rand::thread_rng();
    let total_w = r.columns * r.size;
    let total_h = r.rows * r.size;
    let scale = r.size as f64 / COMPONENT_UNIT as f64;
    let half = r.size as f64 / 2.0;

    let approx = (r.rows * r.columns) as usize * 256;
    let mut body = String::with_capacity(approx);

    body.push_str(&metadata_block(r));

    if let Some(bg) = &r.background {
        body.push_str(&format!(
            "  <rect width=\"{total_w}\" height=\"{total_h}\" fill=\"{}\"/>\n",
            normalize_color(bg)
        ));
    }

    for row in 0..r.rows {
        for col in 0..r.columns {
            let component = components.choose(&mut rng).expect("components non-empty");
            let phase = if r.method == Method::Grid {
                rng.gen_range(0.0..1.0)
            } else {
                cell_phase(r.method, col, row, r.columns, r.rows)
            };
            let resolved = substitute_colors_phased(component, &r.colors, r.method, phase, &mut rng);
            let cx = col as f64 * r.size as f64 + half;
            let cy = row as f64 * r.size as f64 + half;
            let rot = pick_rotation(r.method, phase, &mut rng);
            body.push_str(&format!(
                "  <g transform=\"translate({cx},{cy}) rotate({rot}) scale({scale}) translate(-50,-50)\">\n    {resolved}\n  </g>\n"
            ));
        }
    }

    if r.supersize {
        let count = rng.gen_range(SUPERSIZE_COUNT_RANGE);
        for _ in 0..count {
            let factor = *SUPERSIZE_SCALES.choose(&mut rng).unwrap() as i32;
            let component = components.choose(&mut rng).expect("components non-empty");
            let col = rng.gen_range(1 - factor..r.columns as i32);
            let row = rng.gen_range(1 - factor..r.rows as i32);
            let anchor_col = col.clamp(0, r.columns as i32 - 1) as u32;
            let anchor_row = row.clamp(0, r.rows as i32 - 1) as u32;
            let phase = if r.method == Method::Grid {
                rng.gen_range(0.0..1.0)
            } else {
                cell_phase(r.method, anchor_col, anchor_row, r.columns, r.rows)
            };
            let resolved = substitute_colors_phased(component, &r.colors, r.method, phase, &mut rng);
            let block = factor * r.size as i32;
            let cx = col as f64 * r.size as f64 + block as f64 / 2.0;
            let cy = row as f64 * r.size as f64 + block as f64 / 2.0;
            let big_scale = block as f64 / COMPONENT_UNIT as f64;
            let rot = pick_rotation(r.method, phase, &mut rng);
            body.push_str(&format!(
                "  <g transform=\"translate({cx},{cy}) rotate({rot}) scale({big_scale}) translate(-50,-50)\">\n    {resolved}\n  </g>\n"
            ));
        }
    }

    if r.lines {
        let count = rng.gen_range(LINES_COUNT_RANGE);
        let canvas_w = total_w as f64;
        let canvas_h = total_h as f64;
        let diagonal = (canvas_w * canvas_w + canvas_h * canvas_h).sqrt();
        let min_len = r.size as f64;
        let min_weight = 2.0;
        let max_weight = (r.size as f64 / 2.0).max(min_weight + 1.0);
        let sqrt2 = std::f64::consts::SQRT_2;
        for _ in 0..count {
            let orient = rng.gen_range(0..4);
            let max_len = match orient {
                0 => canvas_w,
                1 => canvas_h,
                _ => diagonal,
            };
            let len = rng.gen_range(min_len..=max_len);
            let weight = rng.gen_range(min_weight..=max_weight);
            let cx = rng.gen_range(0.0..canvas_w);
            let cy = rng.gen_range(0.0..canvas_h);
            let half = len / 2.0;
            let (x1, y1, x2, y2) = match orient {
                0 => (cx - half, cy, cx + half, cy),
                1 => (cx, cy - half, cx, cy + half),
                2 => {
                    let d = half / sqrt2;
                    (cx - d, cy - d, cx + d, cy + d)
                }
                _ => {
                    let d = half / sqrt2;
                    (cx - d, cy + d, cx + d, cy - d)
                }
            };
            let color = r.colors.choose(&mut rng).expect("palette non-empty");
            body.push_str(&format!(
                "  <line x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\" stroke=\"{}\" stroke-width=\"{weight:.1}\" stroke-linecap=\"round\"/>\n",
                normalize_color(color),
            ));
        }
    }

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {total_w} {total_h}\" width=\"{total_w}\" height=\"{total_h}\" style=\"overflow:hidden\">\n{body}</svg>\n"
    )
}

fn default_output_path() -> PathBuf {
    let stamp = Local::now().format("%Y%m%d_%H%M%S");
    PathBuf::from(format!("bauhaus_{stamp}.svg"))
}

fn load_config(path: &Path) -> Result<Config> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading config {}", path.display()))?;
    serde_yml::from_str(&raw).context("parsing yaml config")
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.list_palettes {
        for p in palettes::PALETTES {
            let label = if p.aliases.is_empty() {
                p.name.to_string()
            } else {
                format!("{} (aka {})", p.name, p.aliases.join(", "))
            };
            println!(
                "{:<28} colors: {}  backgrounds: {}",
                label,
                p.colors.join(", "),
                p.backgrounds.join(", ")
            );
        }
        return Ok(());
    }

    let resolved = match (args.theme.as_ref(), args.config.as_ref()) {
        (Some(theme_path), Some(config_path)) => {
            let base = read_theme(theme_path)?;
            let cfg = load_config(config_path)?;
            apply_overrides(base, cfg)?
        }
        (Some(theme_path), None) => read_theme(theme_path)?,
        (None, Some(config_path)) => {
            let cfg = load_config(config_path)?;
            resolve_standalone(cfg)?
        }
        (None, None) => bail!("must provide --config or --theme (or --list-palettes)"),
    };

    let mut components = load_components(&args.assets)?;
    let loaded = components.len();
    if let Some(lim) = resolved.limit {
        let n = (lim as usize).min(components.len());
        let mut rng = rand::thread_rng();
        components.shuffle(&mut rng);
        components.truncate(n);
    }
    eprintln!(
        "loaded {} components from {}{}",
        components.len(),
        args.assets.display(),
        if resolved.limit.is_some() && components.len() < loaded {
            format!(" (limited from {loaded})")
        } else {
            String::new()
        }
    );

    let svg = generate_svg(&resolved, &components);

    let out_path = args.output.unwrap_or_else(default_output_path);
    fs::write(&out_path, &svg)
        .with_context(|| format!("writing {}", out_path.display()))?;
    println!("{}", out_path.display());
    Ok(())
}
