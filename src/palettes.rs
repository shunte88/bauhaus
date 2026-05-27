pub struct BuiltinPalette {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub colors: &'static [&'static str],
    pub backgrounds: &'static [&'static str],
}

pub const PALETTES: &[BuiltinPalette] = &[
    BuiltinPalette {
        name: "Modern Noir",
        aliases: &[],
        colors: &["#FF6B00", "#007065", "#FFC700", "#FF95CA", "#4172D1", "#FFFFFF"],
        backgrounds: &["#1A1A1A", "#000000"],
    },
    BuiltinPalette {
        name: "Exhibition 1923",
        aliases: &[],
        colors: &["#2c2c2c", "#e64f99", "#f6c000", "#3b7b9e", "#e65f3e", "#3c8b67"],
        backgrounds: &["#f3eee1"],
    },
    BuiltinPalette {
        name: "Primary",
        aliases: &[],
        colors: &["#dd1f26", "#f5d600", "#006ab2", "#231f20"],
        backgrounds: &["#e7e7e7", "#ffffff"],
    },
    BuiltinPalette {
        name: "Kandinsky",
        aliases: &[],
        colors: &["#4a90e2", "#f5a623", "#d0021b", "#50e3c2", "#232323"],
        backgrounds: &["#fafafa", "#f0e9d6"],
    },
    BuiltinPalette {
        name: "Sunset",
        aliases: &[],
        colors: &["#F94144", "#F3722C", "#F8961E", "#F9C74F", "#90BE6D"],
        backgrounds: &["#43AA8B", "#577590"],
    },
    BuiltinPalette {
        name: "Ocean",
        aliases: &[],
        colors: &["#001219", "#005f73", "#0a9396", "#94d2bd", "#e9d8a6", "#ee9b00"],
        backgrounds: &["#ca6702", "#bb3e03", "#ae2012", "#9b2226"],
    },
    BuiltinPalette {
        name: "Albers",
        aliases: &[],
        colors: &["#a60303", "#3a54a4", "#f7d80d", "#231f20", "#d88b02"],
        backgrounds: &["#e8e8e8", "#fdfdfd"],
    },
    BuiltinPalette {
        name: "Moholy-Nagy",
        aliases: &[],
        colors: &["#c1272d", "#009245", "#f7931e", "#00a99d", "#2e3192"],
        backgrounds: &["#f1f1f2"],
    },
    BuiltinPalette {
        name: "Spring",
        aliases: &[],
        colors: &["#F9C5D5", "#A3D977", "#FFE066", "#7BC8E2", "#C5A3E0", "#FF8FA3"],
        backgrounds: &["#FFFBF0", "#F5F5DC"],
    },
    BuiltinPalette {
        name: "Summer",
        aliases: &[],
        colors: &["#FFD700", "#FF6B35", "#00B4D8", "#06D6A0", "#FF006E", "#FFFFFF"],
        backgrounds: &["#FFE5B4", "#90E0EF"],
    },
    BuiltinPalette {
        name: "Autumn",
        aliases: &["Fall"],
        colors: &["#D9480F", "#8B2500", "#E1A100", "#6B4423", "#7C8B3D", "#C8552A"],
        backgrounds: &["#F4E4C1", "#5C3A21"],
    },
    BuiltinPalette {
        name: "Winter",
        aliases: &[],
        colors: &["#1E3A5F", "#A7C7E7", "#2E5D3B", "#8B95A1", "#D6E5F0", "#7A0019"],
        backgrounds: &["#F0F4F8", "#0F1419"],
    },
    BuiltinPalette {
        name: "Monochrome",
        aliases: &["Grayscale"],
        colors: &["#000000", "#333333", "#666666", "#999999", "#CCCCCC", "#FFFFFF"],
        backgrounds: &["#F0F0F0", "#FFFFFF"],
    },
    BuiltinPalette {
        name: "Vibrant",
        aliases: &[],
        colors: &["#e6194b", "#3cb44b", "#ffe119", "#4363d8", "#f58231", "#911eb4"],
        backgrounds: &["#42d4f4", "#f032e6"],
    },
    BuiltinPalette {
        name: "Comiskey01",
        aliases: &["barrett01"],
        colors: &["#efefee", "#f7bd18", "#ee6c15", "#1f3a61", "#1a2b43", "#3ca1a1", "#fefefe"],
        backgrounds: &["#efefee", "#000000", "#fefefe"],
    },
    BuiltinPalette {
        name: "Comiskey02",
        aliases: &["barrett02"],
        colors: &["#fefefe", "#40aaaa", "#181c17", "#214345", "#ee6c15", "#f1bb13", "#d8c8aa"],
        backgrounds: &["#d8c8aa", "#000000", "#fefefe"],
    },
];

fn normalize_key(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

pub fn lookup(name: &str) -> Option<&'static BuiltinPalette> {
    let key = normalize_key(name);
    PALETTES.iter().find(|p| {
        normalize_key(p.name) == key
            || p.aliases.iter().any(|a| normalize_key(a) == key)
    })
}

pub fn names() -> Vec<String> {
    PALETTES
        .iter()
        .map(|p| {
            if p.aliases.is_empty() {
                p.name.to_string()
            } else {
                format!("{} (aka {})", p.name, p.aliases.join(", "))
            }
        })
        .collect()
}
