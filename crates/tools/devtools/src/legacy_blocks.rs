//! Extractor for the reference block (`Tile.cpp`) registration table.
//!
//! The reference declares every block as a fluent chain:
//!
//! ```text
//! Tile::stone = (new StoneTile(1)) ->setDestroyTime(1.5f)->setExplodeable(10)
//!     ->setSoundType(Tile::SOUND_STONE)->setIconName(L"stone")
//!     ->setDescriptionId(IDS_TILE_STONE)->setUseDescriptionId(IDS_DESC_STONE);
//! ```
//!
//! [`parse_tile_source`] turns those chains into [`LegacyBlock`] rows, which
//! [`render_toml`] writes as `data/legacy/blocks.toml`. This table exists so
//! the save importer / parity tools can map legacy numeric ids to namespaced
//! ids; the engine itself never uses numeric block ids (SPEC.md 5.2).
//!
//! The parser is deliberately tolerant: anything it does not recognize is
//! skipped (and reported by the CLI), never guessed.

use std::fmt::Write as _;

/// One block as declared by the reference source.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyBlock {
    /// Legacy numeric tile id (`Tile::tiles[id]`).
    pub id: u32,
    /// Reference static member name (`stone`, `treeTrunk`, ...).
    pub symbol: String,
    /// C++ class that implements it (`Tile`, `StoneTile`, `LiquidTileDynamic`, ...).
    pub class: String,
    /// Icon/texture base name from `setIconName`, when declared.
    pub icon: Option<String>,
    /// Hardness from `setDestroyTime` (seconds, `-1` = indestructible), when declared.
    pub destroy_time: Option<f32>,
    /// Blast resistance from `setExplodeable`, when declared.
    pub blast_resistance: Option<f32>,
    /// Light emission from `setLightEmission` (reference stores `0.0..1.0`), when declared.
    pub light_emission: Option<f32>,
    /// Light opacity from `setLightBlock`, when declared.
    pub light_block: Option<i32>,
    /// Localization key source (`IDS_TILE_*`), when declared.
    pub description_id: Option<String>,
    /// Whether the block sends tile data (legacy metadata) to clients.
    pub sends_tile_data: bool,
}

/// Splits `Tile.cpp` into `Tile::<symbol> = ...;` statements.
fn statements(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(statement) = current.as_mut() {
            statement.push(' ');
            statement.push_str(trimmed);
            if trimmed.contains(';') {
                out.push(current.take().expect("statement in progress"));
            }
            continue;
        }
        if trimmed.starts_with("Tile::") && trimmed.contains('=') {
            current = Some(trimmed.to_string());
            if trimmed.contains(';') {
                out.push(current.take().expect("statement in progress"));
            }
        }
    }
    out
}

/// Extracts `<symbol>` from a `Tile::<symbol> = ...` statement.
fn symbol_of(statement: &str) -> Option<&str> {
    let rest = statement.strip_prefix("Tile::")?;
    let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    let symbol = &rest[..end];
    if symbol.is_empty() {
        None
    } else {
        Some(symbol)
    }
}

/// Parses the class name and numeric id from the `new <Class>(<id>` part.
fn class_and_id(statement: &str) -> Option<(String, u32)> {
    let after_new = statement.split_once("new ")?.1;
    let class_end = after_new
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(after_new.len());
    let class = &after_new[..class_end];
    let after_class = after_new[class_end..].trim_start();
    let args = after_class.strip_prefix('(')?.trim_start();
    let digits: String = args.chars().take_while(char::is_ascii_digit).collect();
    if class.is_empty() || digits.is_empty() {
        return None;
    }
    Some((class.to_string(), digits.parse().ok()?))
}

/// Text between `->set<key>(` and the matching `)`.
fn call_arg<'a>(statement: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("->{key}(");
    let start = statement.find(&needle)? + needle.len();
    let rest = &statement[start..];
    let end = rest.find(')')?;
    Some(rest[..end].trim())
}

/// Parses a C++ float literal (`1.5f`, `10.0`, `00.0f`).
fn parse_float(text: &str) -> Option<f32> {
    text.trim().trim_end_matches(['f', 'F']).parse().ok()
}

/// Parses a C++ integer literal.
fn parse_int(text: &str) -> Option<i32> {
    text.trim().trim_end_matches(['f', 'F']).parse().ok()
}

/// Strips a wide-string literal wrapper: `L"stone"` → `stone`.
fn parse_wide_string(text: &str) -> Option<String> {
    let inner = text.trim().strip_prefix("L\"")?.strip_suffix('"')?;
    Some(inner.to_string())
}

/// Parses one statement into a [`LegacyBlock`], or `None` if it is not a block
/// registration (sound types, constants, arrays, ...).
pub fn parse_tile_statement(statement: &str) -> Option<LegacyBlock> {
    let symbol = symbol_of(statement)?.to_string();
    let (class, id) = class_and_id(statement)?;
    // A block registration always sets at least one of these; constants and
    // sound types do not have a numeric id in `new X(<id>`.
    if !statement.contains("setDestroyTime") && !statement.contains("setIconName") {
        return None;
    }
    Some(LegacyBlock {
        id,
        symbol,
        class,
        icon: call_arg(statement, "setIconName").and_then(parse_wide_string),
        destroy_time: call_arg(statement, "setDestroyTime").and_then(parse_float),
        blast_resistance: call_arg(statement, "setExplodeable").and_then(parse_float),
        light_emission: call_arg(statement, "setLightEmission").and_then(parse_float),
        light_block: call_arg(statement, "setLightBlock").and_then(parse_int),
        description_id: call_arg(statement, "setDescriptionId").map(str::to_string),
        sends_tile_data: statement.contains("->sendTileData("),
    })
}

/// Parses every block registration in a `Tile.cpp` source, sorted by legacy id.
///
/// Duplicate ids are reported as an error (the reference must never declare
/// the same numeric id twice).
pub fn parse_tile_source(source: &str) -> Result<Vec<LegacyBlock>, String> {
    let mut blocks: Vec<LegacyBlock> = statements(source)
        .iter()
        .filter_map(|statement| parse_tile_statement(statement))
        .collect();
    blocks.sort_by_key(|block| block.id);
    for pair in blocks.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(format!(
                "duplicate legacy block id {} (`{}` and `{}`)",
                pair[0].id, pair[0].symbol, pair[1].symbol
            ));
        }
    }
    Ok(blocks)
}

/// Formats a float so it stays a TOML float (`10` → `10.0`).
fn float_literal(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

/// Renders the extracted table as deterministic TOML.
pub fn render_toml(blocks: &[LegacyBlock], source_label: &str) -> String {
    let mut out = String::new();
    out.push_str("# Generated by `cargo run -p mclre-devtools --bin extract-legacy`.\n");
    out.push_str("# Do not edit by hand: regenerate from the reference source.\n");
    out.push_str("# Legacy (id) -> block metadata, for save importers and parity tools only.\n");
    let _ = writeln!(out, "# Source: {source_label}");
    let _ = writeln!(out, "\nblock_count = {}", blocks.len());
    for block in blocks {
        out.push_str("\n[[block]]\n");
        let _ = writeln!(out, "id = {}", block.id);
        let _ = writeln!(out, "symbol = \"{}\"", block.symbol);
        let _ = writeln!(out, "class = \"{}\"", block.class);
        if let Some(icon) = &block.icon {
            let _ = writeln!(out, "icon = \"{icon}\"");
        }
        if let Some(value) = block.destroy_time {
            let _ = writeln!(out, "destroy_time = {}", float_literal(value));
        }
        if let Some(value) = block.blast_resistance {
            let _ = writeln!(out, "blast_resistance = {}", float_literal(value));
        }
        if let Some(value) = block.light_emission {
            let _ = writeln!(out, "light_emission = {}", float_literal(value));
        }
        if let Some(value) = block.light_block {
            let _ = writeln!(out, "light_block = {value}");
        }
        if let Some(description) = &block.description_id {
            let _ = writeln!(out, "description_id = \"{description}\"");
        }
        if block.sends_tile_data {
            let _ = writeln!(out, "sends_tile_data = true");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
	static const int maxBuildHeight = 256;
	Tile *Tile::stone = NULL;
	Tile::SoundType *Tile::SOUND_STONE = NULL;

	void Tile::staticCtor()
	{
		Tile::SOUND_STONE = new Tile::SoundType(eMaterialSoundType_STONE, 1, 1);
		Tile::RED_STONE = new Tile::SoundType(eMaterialSoundType_STONE, 1, 1.5f);

		Tile::stone = (new StoneTile(1))->setDestroyTime(1.5f)->setExplodeable(10)->setSoundType(Tile::SOUND_STONE)->setIconName(L"stone")->setDescriptionId(IDS_TILE_STONE)->setUseDescriptionId(IDS_DESC_STONE);
		Tile::grass = (GrassTile *) (new GrassTile(2))->setDestroyTime(0.6f)->setSoundType(Tile::SOUND_GRASS)->setIconName(L"grass")->setDescriptionId(IDS_TILE_GRASS);
		Tile::water = (LiquidTile *)(new LiquidTileDynamic(8, Material::water))->setDestroyTime(100.0f)->setLightBlock(3)->setIconName(L"water_flow")->setDescriptionId(IDS_TILE_WATER)->sendTileData();
		Tile::calmLava = (new LiquidTileStatic(11, Material::lava))	->setDestroyTime(100.0f)->setLightEmission(1.0f)->setLightBlock(255)->setIconName(L"lava_still")->setDescriptionId(IDS_TILE_LAVA);
		Tile::bedrock = (new Tile(7, Material::stone))->setDestroyTime(-1.0f);
		Tile::tiles[1] = Tile::stone;
	}
"#;

    #[test]
    fn parses_simple_and_cast_registrations() {
        let blocks = parse_tile_source(FIXTURE).expect("fixture parses");
        let ids: Vec<u32> = blocks.iter().map(|b| b.id).collect();
        assert_eq!(ids, vec![1, 2, 7, 8, 11]);

        let stone = &blocks[0];
        assert_eq!(stone.symbol, "stone");
        assert_eq!(stone.class, "StoneTile");
        assert_eq!(stone.icon.as_deref(), Some("stone"));
        assert_eq!(stone.destroy_time, Some(1.5));
        assert_eq!(stone.blast_resistance, Some(10.0));
        assert_eq!(stone.description_id.as_deref(), Some("IDS_TILE_STONE"));
        assert!(!stone.sends_tile_data);

        let grass = &blocks[1];
        assert_eq!(grass.class, "GrassTile");
        assert_eq!(grass.destroy_time, Some(0.6));
        assert_eq!(grass.blast_resistance, None);

        let water = &blocks[3];
        assert_eq!(water.class, "LiquidTileDynamic");
        assert_eq!(water.light_block, Some(3));
        assert!(water.sends_tile_data);

        let lava = &blocks[4];
        assert_eq!(lava.light_emission, Some(1.0));
        assert_eq!(lava.light_block, Some(255));

        let bedrock = &blocks[2];
        assert_eq!(bedrock.destroy_time, Some(-1.0));
        assert_eq!(bedrock.icon, None);
    }

    #[test]
    fn ignores_non_block_statements() {
        let source = r#"
            Tile::TILE_DESCRIPTION_PREFIX = L"Tile.";
            Tile::INDESTRUCTIBLE_DESTROY_TIME = -1.0f;
            bool Tile::solid[TILE_NUM_COUNT];
            Tile::solid[Tile::stone_Id] = true;
        "#;
        assert!(parse_tile_source(source).expect("parses").is_empty());
    }

    #[test]
    fn rejects_duplicate_ids() {
        let source = r"
            Tile::a = (new Tile(5, Material::stone))->setDestroyTime(1.0f);
            Tile::b = (new Tile(5, Material::stone))->setDestroyTime(2.0f);
        ";
        let error = parse_tile_source(source).expect_err("duplicate id");
        assert!(error.contains("duplicate legacy block id 5"), "{error}");
    }

    #[test]
    fn rendered_toml_is_deterministic_and_parses() {
        let blocks = parse_tile_source(FIXTURE).expect("fixture parses");
        let toml_a = render_toml(&blocks, "fixture");
        let toml_b = render_toml(&blocks, "fixture");
        assert_eq!(toml_a, toml_b);
        assert!(toml_a.contains("block_count = 5"));
        assert!(toml_a.contains("id = 1\nsymbol = \"stone\"\nclass = \"StoneTile\""));
        assert!(
            toml_a.contains("destroy_time = 10.0") || toml_a.contains("blast_resistance = 10.0")
        );
        assert!(toml_a.contains("sends_tile_data = true"));

        let parsed: toml::Value = toml_a.parse().expect("valid TOML");
        let entries = parsed
            .get("block")
            .and_then(|value| value.as_array())
            .expect("block array");
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn parses_the_real_reference_when_present() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../reference/lce-141217/MinecraftConsoles-141217/Minecraft.World/Tile.cpp"
        );
        let Ok(source) = std::fs::read_to_string(path) else {
            // Reference submodule not checked out (CI): the fixture covers the parser.
            return;
        };
        let blocks = parse_tile_source(&source).expect("reference Tile.cpp parses");
        assert!(blocks.len() > 150, "only parsed {} blocks", blocks.len());
        let stone = blocks
            .iter()
            .find(|block| block.id == 1)
            .expect("stone is id 1");
        assert_eq!(stone.symbol, "stone");
        assert_eq!(stone.icon.as_deref(), Some("stone"));
        // Ids are sorted and unique (enforced by the parser).
        assert!(blocks.windows(2).all(|pair| pair[0].id < pair[1].id));
    }
}
