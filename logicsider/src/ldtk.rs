use super::*;

impl GameState {
    pub fn from_ldtk(ldtk: &ldtk_json::Ldtk, config: &Config, level: usize) -> Self {
        let level = &ldtk.levels[level];
        let tile_by_int: HashMap<u32, Tile> = ldtk
            .defs
            .layers
            .iter()
            .flat_map(|layer| &layer.int_grid_values)
            .filter_map(|value| {
                Some((
                    value.value,
                    match value.identifier.as_str() {
                        "block" => Tile::Block,
                        "cloud" => Tile::Cloud,
                        "disable" => Tile::Disable,
                        _ => return None,
                    },
                ))
            })
            .collect();
        let tiles = level
            .layer_instances
            .iter()
            .filter(|layer| !layer.int_grid_csv.is_empty())
            .flat_map(|layer| {
                layer
                    .int_grid_csv
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_index, value)| *value != 0)
                    .map(|(index, value)| {
                        (
                            vec2(
                                index as i32 % layer.grid_width as i32,
                                -(index as i32 / layer.grid_width as i32),
                            ),
                            tile_by_int[&value].clone(),
                        )
                    })
            })
            .collect();
        let mut result = Self {
            id_gen: id::Gen::new(),
            tiles,
            entities: default(),
            powerups: default(),
            goals: default(),
            selected_player: None,
            stable: false,
        };
        for entity in level
            .layer_instances
            .iter()
            .flat_map(|layer| &layer.entity_instances)
        {
            let pos = Position::from_ldtk_entity(
                entity,
                if entity.identifier.starts_with("Power") {
                    IntAngle::DOWN
                } else {
                    IntAngle::RIGHT
                },
            );
            if let Some(effect) = entity.identifier.strip_suffix("Power") {
                result.powerups.insert(Powerup {
                    id: result.id_gen.gen(),
                    effect: Effect::from_str(effect),
                    pos: Position::from_ldtk_entity(entity, IntAngle::DOWN),
                });
            } else {
                match entity.identifier.as_str() {
                    "Goal" => {
                        result.goals.insert(Goal {
                            id: result.id_gen.gen(),
                            pos: Position::from_ldtk_entity(entity, IntAngle::RIGHT),
                        });
                    }
                    entity_name => {
                        result.add_entity(
                            entity_name,
                            config
                                .entities
                                .get(entity_name)
                                .expect(&format!("Entity {entity_name:?} unimplemented")),
                            pos,
                        );
                    }
                }
            }
        }
        result.selected_player = result
            .entities
            .iter()
            .filter(|entity| entity.properties.player)
            .min_by_key(|entity| entity.id.raw())
            .map(|entity| entity.id);
        result
    }
}
