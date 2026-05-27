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
const SUPERSIZE_COUNT_RANGE: std::ops::RangeInclusive<u32> = 2..=5;

#[derive(Parser, Debug)]
#[command(name = "bauhaus", about = "Generate Bauhaus-style SVG patterns")]
struct Args {
    #[arg(short, long, required_unless_present = "list_palettes")]
    config: Option<PathBuf>,

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

#[derive(Deserialize, Debug)]
struct Config {
    palette: PaletteSpec,
    #[serde(default)]
    background: Option<String>,
    #[serde(default)]
    supersize: bool,
    columns: u32,
    rows: u32,
    size: u32,
}

struct Resolved {
    colors: Vec<String>,
    background: Option<String>,
    supersize: bool,
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

fn resolve(cfg: Config) -> Result<Resolved> {
    let mut rng = rand::thread_rng();

    let (colors, bg_from_palette) = match cfg.palette {
        PaletteSpec::Named(name) => {
            let p = palettes::lookup(&name).ok_or_else(|| {
                anyhow!(
                    "unknown palette {name:?}; available: {}",
                    palettes::names().join(", ")
                )
            })?;
            let colors: Vec<String> = p.colors.iter().map(|s| s.to_string()).collect();
            let bg = p
                .backgrounds
                .choose(&mut rng)
                .map(|s| s.to_string());
            (colors, bg)
        }
        PaletteSpec::Inline(v) => (v, None),
    };

    if colors.is_empty() || colors.len() > 10 {
        bail!("palette must contain between 1 and 10 colors (got {})", colors.len());
    }
    for c in &colors {
        validate_hex(c)?;
    }
    if let Some(bg) = &cfg.background {
        validate_hex(bg)?;
    }
    if !(4..=20).contains(&cfg.columns) {
        bail!("columns must be between 4 and 20 (got {})", cfg.columns);
    }
    if !(4..=20).contains(&cfg.rows) {
        bail!("rows must be between 4 and 20 (got {})", cfg.rows);
    }
    if !(20..=50).contains(&cfg.size) {
        bail!("size must be between 20 and 50 (got {})", cfg.size);
    }

    Ok(Resolved {
        colors,
        background: cfg.background.or(bg_from_palette),
        supersize: cfg.supersize,
        columns: cfg.columns,
        rows: cfg.rows,
        size: cfg.size,
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

fn substitute_colors(template: &str, palette: &[String], rng: &mut impl Rng) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(idx) = rest.find(COLOR_TOKEN) {
        out.push_str(&rest[..idx]);
        let color = palette.choose(rng).expect("palette validated non-empty");
        out.push_str(&normalize_color(color));
        rest = &rest[idx + COLOR_TOKEN.len()..];
    }
    out.push_str(rest);
    out
}

fn generate_svg(r: &Resolved, components: &[String]) -> String {
    let mut rng = rand::thread_rng();
    let total_w = r.columns * r.size;
    let total_h = r.rows * r.size;
    let scale = r.size as f64 / COMPONENT_UNIT as f64;
    let half = r.size as f64 / 2.0;

    let approx = (r.rows * r.columns) as usize * 256;
    let mut body = String::with_capacity(approx);

    if let Some(bg) = &r.background {
        body.push_str(&format!(
            "  <rect width=\"{total_w}\" height=\"{total_h}\" fill=\"{}\"/>\n",
            normalize_color(bg)
        ));
    }

    for row in 0..r.rows {
        for col in 0..r.columns {
            let component = components.choose(&mut rng).expect("components non-empty");
            let resolved = substitute_colors(component, &r.colors, &mut rng);
            let cx = col as f64 * r.size as f64 + half;
            let cy = row as f64 * r.size as f64 + half;
            let rot = *ROTATIONS.choose(&mut rng).unwrap();
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
            let resolved = substitute_colors(component, &r.colors, &mut rng);
            let col = rng.gen_range(1 - factor..r.columns as i32);
            let row = rng.gen_range(1 - factor..r.rows as i32);
            let block = factor * r.size as i32;
            let cx = col as f64 * r.size as f64 + block as f64 / 2.0;
            let cy = row as f64 * r.size as f64 + block as f64 / 2.0;
            let big_scale = block as f64 / COMPONENT_UNIT as f64;
            let rot = *ROTATIONS.choose(&mut rng).unwrap();
            body.push_str(&format!(
                "  <g transform=\"translate({cx},{cy}) rotate({rot}) scale({big_scale}) translate(-50,-50)\">\n    {resolved}\n  </g>\n"
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

    let config_path = args.config.expect("clap enforces presence");
    let raw = fs::read_to_string(&config_path)
        .with_context(|| format!("reading config {}", config_path.display()))?;
    let cfg: Config = serde_yml::from_str(&raw).context("parsing yaml config")?;
    let resolved = resolve(cfg)?;

    let components = load_components(&args.assets)?;
    eprintln!(
        "loaded {} components from {}",
        components.len(),
        args.assets.display()
    );

    let svg = generate_svg(&resolved, &components);

    let out_path = args.output.unwrap_or_else(default_output_path);
    fs::write(&out_path, &svg)
        .with_context(|| format!("writing {}", out_path.display()))?;
    println!("{}", out_path.display());
    Ok(())
}
