use geng::prelude::{itertools::Itertools, *};
use std::rc::Rc;
pub mod json;

// TODO: remove and just use Mesh
pub struct Texture {
    pub atlas: Rc<ugli::Texture>,
    pub uvs: Aabb2<f32>,
}

pub struct Entity {
    pub identifier: String,
    pub pos: vec2<i32>,
    pub texture: Rc<Texture>,
}

pub struct Mesh {
    pub vertex_data: ugli::VertexBuffer<draw2d::TexturedVertex>,
    pub texture: Rc<ugli::Texture>,
}

pub struct Layer {
    pub entities: Vec<Entity>,
    pub mesh: Option<Mesh>,
}

pub struct Level {
    pub identifier: String,
    pub layers: Vec<Layer>,
}

pub struct Ldtk {
    pub levels: Vec<Level>,
}

impl geng::asset::Load for Ldtk {
    fn load(manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        let manager = manager.clone();
        let path = path.to_owned();
        async move {
            let manager = &manager;
            let base_path = path.parent().unwrap();
            let json: json::Ldtk = file::load_json(&path).await?;
            struct TilesetDef {
                texture: Rc<ugli::Texture>,
                tile_size: f32,
            }
            let tilesets: HashMap<i64, TilesetDef> =
                future::join_all(json.defs.tilesets.into_iter().map(|tileset| async move {
                    Ok::<_, anyhow::Error>((
                        tileset.uid,
                        TilesetDef {
                            texture: match tileset.rel_path {
                                Some(path) => {
                                    let mut texture: ugli::Texture =
                                        manager.load(base_path.join(path)).await?;
                                    texture.set_filter(ugli::Filter::Nearest);
                                    Rc::new(texture)
                                }
                                None => Rc::new(ugli::Texture::new_with(
                                    manager.ugli(),
                                    vec2(1, 1),
                                    |_| Rgba::TRANSPARENT_BLACK,
                                )),
                            },
                            tile_size: tileset.tile_grid_size as f32,
                        },
                    ))
                }))
                .await
                .into_iter()
                .try_collect()?;
            struct EntityDef {
                texture: Rc<Texture>,
            }
            let entities: HashMap<String, EntityDef> = json
                .defs
                .entities
                .into_iter()
                .map(|entity| {
                    let tileset = &tilesets[&entity.tileset_id];
                    (
                        entity.identifier,
                        EntityDef {
                            texture: Rc::new(Texture {
                                atlas: tileset.texture.clone(),
                                uvs: {
                                    let mut uvs =
                                        Aabb2::point(vec2(entity.tile_rect.x, entity.tile_rect.y))
                                            .extend_positive(vec2(
                                                entity.tile_rect.w,
                                                entity.tile_rect.h,
                                            ))
                                            .map(|x| x as f32)
                                            .map_bounds(|v| {
                                                v / tileset.texture.size().map(|x| x as f32)
                                            })
                                            .map_bounds(|v| vec2(v.x, 1.0 - v.y));
                                    mem::swap(&mut uvs.min.y, &mut uvs.max.y);
                                    uvs
                                },
                            }),
                        },
                    )
                })
                .collect();
            Ok(Self {
                levels: json
                    .levels
                    .into_iter()
                    .map(|level| Level {
                        identifier: level.identifier,
                        layers: level
                            .layer_instances
                            .into_iter()
                            .map(|layer| Layer {
                                entities: layer
                                    .entity_instances
                                    .into_iter()
                                    .map(|entity| Entity {
                                        pos: vec2(entity.grid.x, -entity.grid.y),
                                        texture: entities[&entity.identifier].texture.clone(),
                                        identifier: entity.identifier,
                                    })
                                    .collect(),
                                mesh: if !layer.auto_layer_tiles.is_empty() {
                                    let tileset = &tilesets[&layer
                                        .tileset_def_uid
                                        .expect("tileset uid not set for autotiled layer")];
                                    Some(Mesh {
                                        vertex_data: ugli::VertexBuffer::new_static(
                                            manager.ugli(),
                                            layer
                                                .auto_layer_tiles
                                                .into_iter()
                                                .flat_map(|tile| {
                                                    let uv = vec2(
                                                        tile.src.x as f32,
                                                        tileset.texture.size().y as f32
                                                            - tileset.tile_size
                                                            - tile.src.y as f32,
                                                    ) / tileset
                                                        .texture
                                                        .size()
                                                        .map(|x| x as f32);
                                                    let uv_size = vec2::splat(tileset.tile_size)
                                                        / tileset.texture.size().map(|x| x as f32);

                                                    let pos = vec2(tile.px.x, -tile.px.y)
                                                        .map(|x| x as f32)
                                                        / tileset.tile_size;

                                                    // !hellobadcop
                                                    let color = Rgba::new(1.0, 1.0, 1.0, tile.a);
                                                    let quad = [
                                                        draw2d::TexturedVertex {
                                                            a_pos: pos,
                                                            a_vt: uv,
                                                            a_color: color,
                                                        },
                                                        draw2d::TexturedVertex {
                                                            a_pos: pos + vec2(1.0, 0.0),
                                                            a_vt: uv + vec2(uv_size.x, 0.0),
                                                            a_color: color,
                                                        },
                                                        draw2d::TexturedVertex {
                                                            a_pos: pos + vec2(1.0, 1.0),
                                                            a_vt: uv + uv_size,
                                                            a_color: color,
                                                        },
                                                        draw2d::TexturedVertex {
                                                            a_pos: pos + vec2(0.0, 1.0),
                                                            a_vt: uv + vec2(0.0, uv_size.y),
                                                            a_color: color,
                                                        },
                                                    ];
                                                    [
                                                        quad[0], quad[1], quad[2], quad[0],
                                                        quad[2], quad[3],
                                                    ]
                                                })
                                                .collect(),
                                        ),
                                        texture: tileset.texture.clone(),
                                    })
                                } else {
                                    None
                                },
                            })
                            .collect(),
                    })
                    .collect(),
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("ldtk");
}
