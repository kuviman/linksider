use geng::prelude::{itertools::Itertools, *};
use std::rc::Rc;

pub struct Mesh {
    pub vertex_data: ugli::VertexBuffer<draw2d::TexturedVertex>,
    pub texture: Rc<ugli::Texture>,
}

pub struct Layer {
    pub mesh: Option<Rc<Mesh>>,
}

pub struct Level {
    pub layers: Vec<Layer>,
}

pub struct EntityDef {
    pub mesh: Rc<Mesh>,
}

pub struct Ldtk {
    pub entity_defs: HashMap<String, EntityDef>,
    pub levels: Vec<Rc<Level>>,
    pub json: ldtk_json::Ldtk,
}

fn quad(pos: Aabb2<f32>, uv: Aabb2<f32>, color: Rgba<f32>) -> [draw2d::TexturedVertex; 6] {
    let v = |f: &dyn Fn(Aabb2<f32>) -> vec2<f32>| draw2d::TexturedVertex {
        a_pos: f(pos),
        a_vt: f(uv),
        a_color: color,
    };
    let quad = [
        v(&|x| x.bottom_left()),
        v(&|x| x.bottom_right()),
        v(&|x| x.top_right()),
        v(&|x| x.top_left()),
    ];
    [quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]
}

impl geng::asset::Load for Ldtk {
    fn load(manager: &geng::asset::Manager, path: &std::path::Path) -> geng::asset::Future<Self> {
        let manager = manager.clone();
        let path = path.to_owned();
        async move {
            let manager = &manager;
            let base_path = path.parent().unwrap();
            let json: ldtk_json::Ldtk = file::load_json(&path).await?;
            struct TilesetDef {
                texture: Rc<ugli::Texture>,
                tile_size: f32,
            }
            let tilesets: HashMap<i64, TilesetDef> =
                future::join_all(json.defs.tilesets.iter().map(|tileset| async move {
                    Ok::<_, anyhow::Error>((
                        tileset.uid,
                        TilesetDef {
                            texture: match &tileset.rel_path {
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
            let entity_defs: HashMap<String, EntityDef> = json
                .defs
                .entities
                .iter()
                .map(|entity| {
                    let tileset = &tilesets[&entity.tileset_id];
                    (
                        entity.identifier.clone(),
                        EntityDef {
                            mesh: Rc::new(Mesh {
                                texture: tileset.texture.clone(),
                                vertex_data: ugli::VertexBuffer::new_static(
                                    manager.ugli(),
                                    {
                                        let mut uvs = Aabb2::point(vec2(
                                            entity.tile_rect.x,
                                            entity.tile_rect.y,
                                        ))
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
                                        quad(
                                            Aabb2::ZERO.extend_positive(vec2::splat(1.0)),
                                            uvs,
                                            Rgba::WHITE,
                                        )
                                    }
                                    .into(),
                                ),
                            }),
                        },
                    )
                })
                .collect();
            Ok(Self {
                levels: json
                    .levels
                    .iter()
                    .map(|level| Level {
                        layers: level
                            .layer_instances
                            .iter()
                            .map(|layer| Layer {
                                mesh: if !layer.auto_layer_tiles.is_empty() {
                                    let tileset = &tilesets[&layer
                                        .tileset_def_uid
                                        .expect("tileset uid not set for autotiled layer")];
                                    Some(Rc::new(Mesh {
                                        vertex_data: ugli::VertexBuffer::new_static(
                                            manager.ugli(),
                                            layer
                                                .auto_layer_tiles
                                                .iter()
                                                .flat_map(|tile| {
                                                    let uv = vec2(
                                                        tile.src[0] as f32,
                                                        tileset.texture.size().y as f32
                                                            - tileset.tile_size
                                                            - tile.src[1] as f32,
                                                    ) / tileset
                                                        .texture
                                                        .size()
                                                        .map(|x| x as f32);
                                                    let uv_size = vec2::splat(tileset.tile_size)
                                                        / tileset.texture.size().map(|x| x as f32);

                                                    let pos = vec2(tile.px[0], -tile.px[1])
                                                        .map(|x| x as f32)
                                                        / tileset.tile_size;

                                                    // !hellobadcop
                                                    let color = Rgba::new(1.0, 1.0, 1.0, tile.a);
                                                    quad(
                                                        Aabb2::point(pos)
                                                            .extend_positive(vec2::splat(1.0)),
                                                        Aabb2::point(uv).extend_positive(uv_size),
                                                        color,
                                                    )
                                                })
                                                .collect(),
                                        ),
                                        texture: tileset.texture.clone(),
                                    }))
                                } else {
                                    None
                                },
                            })
                            .collect(),
                    })
                    .map(Rc::new)
                    .collect(),
                entity_defs,
                json,
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("ldtk");
}
