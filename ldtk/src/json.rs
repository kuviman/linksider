use batbox_la::*;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TileRect {
    pub tileset_uid: i64,
    pub w: i32,
    pub h: i32,
    pub x: i32,
    pub y: i32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EntityDefinition {
    pub identifier: String,
    pub tile_rect: TileRect,
    pub tileset_id: i64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TilesetDefinition {
    #[serde(rename = "__cWid")]
    pub grid_width: usize,
    #[serde(rename = "__cHei")]
    pub grid_height: usize,
    #[serde(rename = "pxWid")]
    pub pixel_width: usize,
    #[serde(rename = "pxHei")]
    pub pixel_height: usize,
    pub tile_grid_size: i32,
    pub uid: i64,
    pub rel_path: Option<std::path::PathBuf>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IntGridValue {
    pub identifier: String,
    pub value: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LayerDefinition {
    pub identifier: String,
    pub int_grid_values: Vec<IntGridValue>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Definitions {
    pub entities: Vec<EntityDefinition>,
    pub tilesets: Vec<TilesetDefinition>,
    pub layers: Vec<LayerDefinition>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FieldInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__value")]
    pub value: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EntityInstance {
    #[serde(rename = "__grid")]
    pub grid: vec2<i32>,
    #[serde(rename = "__identifier")]
    pub identifier: String,
    pub field_instances: Vec<FieldInstance>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TileInstance {
    /// Alpha/opacity of the tile (0-1, defaults to 1)
    pub a: f32,
    /// "Flip bits", a 2-bits integer to represent the mirror transformations of the tile.
    /// - Bit 0 = X flip
    /// - Bit 1 = Y flip
    /// Examples: f=0 (no flip), f=1 (X flip only), f=2 (Y flip only), f=3 (both flips)
    pub f: u8,
    /// Pixel coordinates of the tile in the layer ([x,y] format). Don't forget optional layer offsets, if they exist!
    pub px: vec2<i32>,
    /// Pixel coordinates of the tile in the tileset ([x,y] format)
    pub src: vec2<i32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LayerInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__cWid")]
    pub grid_width: usize,
    #[serde(rename = "__cHei")]
    pub grid_height: usize,
    #[serde(rename = "__tilesetDefUid")]
    pub tileset_def_uid: Option<i64>,
    pub entity_instances: Vec<EntityInstance>,
    pub auto_layer_tiles: Vec<TileInstance>,
    pub int_grid_csv: Vec<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    pub identifier: String,
    pub layer_instances: Vec<LayerInstance>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Ldtk {
    pub defs: Definitions,
    pub levels: Vec<Level>,
}

#[test]
fn test_load() {
    let file = std::fs::File::open(
        std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join("assets")
            .join("world.ldtk"),
    )
    .unwrap();
    let reader = std::io::BufReader::new(file);
    let ldtk: Ldtk = serde_json::from_reader(reader).unwrap();
    eprintln!("{ldtk:#?}");
}
