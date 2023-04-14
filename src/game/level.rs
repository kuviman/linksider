use super::*;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);

        app.insert_resource(LevelSelection::Index(0));
        app.insert_resource(LdtkSettings {
            set_clear_color: SetClearColor::FromLevelBackground,
            ..Default::default()
        });

        app.add_system(level_restart);
        app.add_system(change_level_cheats);

        app.register_ldtk_int_cell::<BlockBundle>(1);
        app.register_ldtk_int_cell::<DisableBundle>(6);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        LdtkWorldBundle {
            ldtk_handle: asset_server.load("world.ldtk"),
            // transform: Transform::from_scale(Vec3::splat(1.0 / 8.0)),
            ..default()
        },
        Name::new("World"),
    ));
}

#[derive(Default, Component)]
pub struct Blocking;

#[derive(Bundle, LdtkIntCell)]
struct BlockBundle {
    blocking: Blocking,
    trigger: side::Trigger,
}

#[derive(Bundle, LdtkIntCell)]
struct DisableBundle {
    blocking: Blocking,
}

fn level_restart(
    ldtk_worlds: Query<Entity, With<Handle<LdtkAsset>>>,
    input: Res<Input<KeyCode>>,
    mut commands: Commands,
) {
    if input.any_just_released([KeyCode::R, KeyCode::Back]) {
        let ldtk_world = ldtk_worlds.single();
        commands.entity(ldtk_world).insert(Respawn);
    }
}

/// Cheat codes for skipping levels
fn change_level_cheats(input: Res<Input<KeyCode>>, mut level: ResMut<LevelSelection>) {
    let mut dir: isize = 0;
    if input.just_pressed(KeyCode::LBracket) {
        dir -= 1;
    }
    if input.just_pressed(KeyCode::RBracket) {
        dir += 1;
    }
    if dir != 0 {
        match *level {
            LevelSelection::Index(ref mut index) => {
                *index = (*index as isize + dir).max(0) as usize;
            }
            _ => unreachable!(),
        }
    }
}
