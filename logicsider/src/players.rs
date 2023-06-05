use super::*;

impl GameState {
    pub fn player_ids(&self) -> impl Iterator<Item = Id> {
        let mut player_ids: Vec<Id> = self
            .entities
            .iter()
            .filter_map(|entity| {
                if entity.properties.player {
                    Some(entity.id)
                } else {
                    None
                }
            })
            .collect();
        player_ids.sort_by_key(|id| id.raw());
        player_ids.into_iter()
    }

    pub fn selected_player_index(&self) -> Option<usize> {
        self.player_ids()
            .position(|id| Some(id) == self.selected_player)
    }

    pub fn select_player(&mut self, index: usize) {
        self.selected_player = self.player_ids().nth(index)
    }

    pub fn change_player_selection(&mut self, config: &Config, delta: isize) {
        if !config.allow_unstable_player_selection && !self.stable {
            return;
        }
        let Some(current_index) =  self.selected_player_index() else {
            self.select_player(0);
            return;
        };
        let players_count = self.player_ids().count();
        let mut new = current_index as isize + delta;
        if new < 0 {
            new = players_count as isize - 1;
        } else if new >= players_count as isize {
            new = 0;
        }
        self.select_player(new as usize);
    }

    pub fn selected_entity(&self) -> Option<&Entity> {
        self.selected_player.and_then(|id| self.entities.get(&id))
    }
}
