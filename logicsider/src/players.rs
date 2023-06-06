use super::*;

impl GameState {
    pub fn player_ids(&self) -> impl Iterator<Item = Id> + '_ {
        let mut players: Vec<&Entity> = self
            .entities
            .iter()
            .filter(|entity| entity.properties.player)
            .collect();
        players.sort_by_key(|player| player.index);
        players.into_iter().map(|player| player.id)
    }

    pub fn selected_player_index(&self) -> Option<usize> {
        self.player_ids()
            .position(|id| Some(id) == self.selected_player)
    }

    pub fn select_player(&mut self, index: usize) {
        let new = self.player_ids().nth(index);
        self.selected_player = new;
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
