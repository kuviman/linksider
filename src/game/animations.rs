use super::*;

pub fn init(app: &mut App) {
    app.add_system(load_atlas);
    app.add_system(animate);
}

#[derive(Bundle, Clone)]
pub struct AnimationBundle {
    atlas_path: AtlasPath,
    sprite_sheet: SpriteSheetBundle,
    animation_timer: AnimationTimer,
    indices: AnimationIndices,
}

#[derive(Component, Clone)]
struct AtlasPath(&'static str, f32);

fn load_atlas(
    levels: Query<Entity, With<Handle<LdtkLevel>>>,
    mut query: Query<(Entity, &mut Handle<TextureAtlas>, &AtlasPath), Added<AtlasPath>>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut commands: Commands,
) {
    for (entity, mut handle, path) in query.iter_mut() {
        let texture_handle = asset_server.load(path.0);
        let texture_atlas =
            TextureAtlas::from_grid(texture_handle, Vec2::new(path.1, path.1), 1, 5, None, None);
        *handle = texture_atlases.add(texture_atlas);
        if let Ok(level) = levels.get_single() {
            commands.entity(level).add_child(entity);
        }
    }
}

impl AnimationBundle {
    pub fn new(
        coords: GridCoords,
        rot: i32,
        atlas: &'static str,
        atlas_size: Option<f32>,
        top: bool,
        mirror: bool,
    ) -> Self {
        const FRAMES: usize = 5;
        Self {
            atlas_path: AtlasPath(atlas, atlas_size.unwrap_or(16.0)),
            sprite_sheet: SpriteSheetBundle {
                sprite: TextureAtlasSprite {
                    flip_x: mirror,
                    ..default()
                },
                transform: Transform::from_rotation(Quat::from_rotation_z(rot as f32 * PI / 2.0))
                    .with_translation(
                        grid_coords_to_translation(coords, IVec2::new(16, 16)).extend(if top {
                            234.5
                        } else {
                            23.5 // KEKW
                        }),
                    ),
                ..default()
            },
            indices: AnimationIndices {
                first: 0,
                last: FRAMES - 1,
            },
            animation_timer: AnimationTimer(Timer::from_seconds(
                0.3 / FRAMES as f32,
                TimerMode::Repeating,
            )),
        }
    }
}

#[derive(Debug, Component, Clone)]
struct AnimationIndices {
    #[allow(dead_code)] // LOL
    first: usize,
    last: usize,
}

#[derive(Debug, Component, Deref, DerefMut, Clone)]
struct AnimationTimer(Timer);

fn animate(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &AnimationIndices,
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
    )>,
    mut commands: Commands,
) {
    for (entity, indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            if sprite.index == indices.last {
                info!("DESPAWN {entity:?}");
                commands.entity(entity).despawn();
            } else {
                sprite.index += 1;
            };
        }
    }
}
