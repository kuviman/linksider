use geng::prelude::*;
use image::GenericImageView;

pub struct Tileset {
    pub texture: ugli::Texture,
    pub def: TilesetDef,
}

#[derive(Debug)]
pub struct TilesetDef {
    pub tiles: HashMap<String, Tile>,
}

pub trait TileMap {
    type NonEmptyIter<'a>: Iterator<Item = vec2<i32>> + 'a
    where
        Self: 'a;
    fn non_empty_tiles(&self) -> Self::NonEmptyIter<'_>;
    fn get_at(&self, pos: vec2<i32>) -> Option<&str>; // TODO not &str
}

#[derive(Clone, Debug)]
pub struct TexturedTile {
    pub pos: vec2<i32>,
    pub uv: Aabb2<f32>,
}

impl TilesetDef {
    pub fn generate_mesh<'a>(
        &'a self,
        tile_map: &'a impl TileMap,
    ) -> impl Iterator<Item = TexturedTile> + 'a {
        tile_map
            .non_empty_tiles()
            .flat_map(|pos| tile_map.get_at(pos).map(move |value| (pos, value)))
            .flat_map(|(pos, value)| {
                let uv = self
                    .tiles
                    .get(value)
                    .expect(&format!("No def for tile type {value:?}"))
                    .uv(|delta| match tile_map.get_at(pos + delta) {
                        Some(other) => {
                            if other == value {
                                Connection::Same
                            } else {
                                Connection::Different
                            }
                        }
                        None => Connection::Empty,
                    });
                uv.map(|uv| TexturedTile { pos, uv })
            })
    }
}

#[test]
fn test() {
    let (config, def) = futures::executor::block_on(TilesetDef::load(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/tileset/config.ron"),
    ))
    .unwrap();

    struct Map(HashMap<vec2<i32>, &'static str>);
    impl TileMap for Map {
        type NonEmptyIter<'a> = Box<dyn Iterator<Item = vec2<i32>> + 'a>;
        fn non_empty_tiles(&self) -> Self::NonEmptyIter<'_> {
            Box::new(self.0.keys().copied())
        }
        fn get_at(&self, pos: vec2<i32>) -> Option<&str> {
            self.0.get(&pos).copied()
        }
    }
    eprintln!("{def:#?}");
    let mut map = Map(HashMap::new());
    map.0.insert(vec2(0, 0), "block");
    map.0.insert(vec2(1, 0), "block");
    let mesh: HashMap<vec2<i32>, Aabb2<f32>> = def
        .generate_mesh(&map)
        .map(|tile| (tile.pos, tile.uv))
        .collect();
    assert_eq!(
        map.0.keys().collect::<HashSet<_>>(),
        mesh.keys().collect::<HashSet<_>>(),
    );
}

#[derive(Debug)]
pub enum Tile {
    AutoTiled { rules: Vec<Rule> },
    Static { uv: Aabb2<f32> },
}

impl Tile {
    pub fn uv(&self, f: impl Fn(vec2<i32>) -> Connection) -> Option<Aabb2<f32>> {
        match self {
            Self::Static { uv } => Some(*uv),
            Self::AutoTiled { rules } => {
                let matched_rules = rules.iter().filter(|rule| {
                    rule.connections
                        .iter()
                        .all(|(delta, filter)| filter.matches(f(*delta)))
                });
                // let matched_rules = matched_rules.collect::<Vec<_>>();
                matched_rules.choose(&mut thread_rng()).map(|rule| rule.uv)
            }
        }
    }
}

#[derive(Debug)]
pub struct Rule {
    connections: HashMap<vec2<i32>, ConnectionFilter>,
    uv: Aabb2<f32>,
}

pub enum Connection {
    Empty,
    Same,
    Different,
}

#[derive(Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub enum ConnectionFilter {
    Anything,
    Empty,
    NotEmpty,
    Same,
    Different,
}

impl ConnectionFilter {
    pub fn matches(&self, connection: Connection) -> bool {
        match self {
            Self::NotEmpty => !matches!(connection, Connection::Empty),
            Self::Empty => matches!(connection, Connection::Empty),
            Self::Anything => true,
            Self::Same => matches!(connection, Connection::Same),
            Self::Different => !matches!(connection, Connection::Same),
        }
    }
}

type ColorRules = HashMap<Rgba<u8>, Option<ConnectionFilter>>;
static COLOR_RULES: std::sync::OnceLock<ColorRules> = std::sync::OnceLock::new();
fn color_rules() -> &'static ColorRules {
    COLOR_RULES.get_or_init(|| serde_json::from_str(include_str!("color_rules.json")).unwrap())
}

#[test]
fn test_color_rules_parse() {
    assert_eq!(color_rules()[&Rgba::WHITE], Some(ConnectionFilter::Empty));
    assert_eq!(color_rules()[&Rgba::BLACK], None);
}

impl ConnectionFilter {
    fn from_color(color: Rgba<u8>) -> Option<Self> {
        *color_rules()
            .get(&color)
            .expect(&format!("No rule for color {color:?}"))
    }
}

async fn load_rules_from_image(
    path: impl AsRef<std::path::Path>,
    config: &Config,
) -> anyhow::Result<Vec<Rule>> {
    let bytes = file::load_bytes(path).await?;
    let image = image::load_from_memory(&bytes)?;
    let mut result = Vec::new();
    for x in (0..image.width()).step_by(config.tile_size.x) {
        for y in (0..image.height()).step_by(config.tile_size.y) {
            let tile = image::GenericImageView::view(
                &image,
                x,
                y,
                config.tile_size.x as u32,
                config.tile_size.y as u32,
            );
            let mut connections = HashMap::new();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let delta = vec2(dx, dy);
                    let pos = delta.zip(config.tile_size).map(|(d, size)| match d {
                        -1 => 0,
                        0 => size / 2,
                        1 => size - 1,
                        _ => unreachable!(),
                    });
                    let image::Rgba([r, g, b, a]) = tile.get_pixel(pos.x as u32, pos.y as u32);
                    let color = Rgba { r, g, b, a };
                    if a == 0 {
                        continue;
                    }
                    if let Some(connection) = ConnectionFilter::from_color(color) {
                        // Invert y because of different coordinate system in geng/image
                        connections.insert(vec2(delta.x, -delta.y), connection);
                    }
                }
            }
            if !connections.is_empty() {
                result.push(Rule {
                    connections,
                    uv: Aabb2::point(vec2(x, image.height() - y - config.tile_size.y as u32))
                        .extend_positive(config.tile_size.map(|x| x as u32))
                        .map(|x| x as f32)
                        .map_bounds(|v| v / vec2(image.width() as f32, image.height() as f32)),
                });
            }
        }
    }
    Ok(result)
}

#[derive(Deserialize)]
pub struct Config {
    pub texture: std::path::PathBuf,
    pub tile_size: vec2<usize>,
    pub tiles: HashMap<String, TileConfig>,
}

#[derive(Deserialize)]
pub enum TileConfig {
    AutoTile(std::path::PathBuf),
    At(usize, usize),
}

impl TilesetDef {
    pub async fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<(Config, Self)> {
        let path = path.as_ref();
        let base_path = path.parent().unwrap();
        let config: Config = file::load_detect(path).await?;
        let mut tiles = HashMap::new();
        for (name, tile) in &config.tiles {
            tiles.insert(
                name.clone(),
                match tile {
                    TileConfig::AutoTile(path) => {
                        let rules = load_rules_from_image(base_path.join(path), &config).await?;
                        Tile::AutoTiled { rules }
                    }
                    TileConfig::At(x, y) => {
                        // TODO
                        continue;
                    }
                },
            );
        }
        for (name, pos) in &config.tiles {
            // TODO static
        }
        Ok((config, Self { tiles }))
    }
}

impl geng::asset::Load for Tileset {
    fn load(manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        let manager = manager.to_owned();
        let path = path.to_owned();
        async move {
            let (config, def) = TilesetDef::load(path.join("config.ron")).await?;
            let mut texture: ugli::Texture = manager.load(path.join(config.texture)).await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Self { texture, def })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = None;
}
