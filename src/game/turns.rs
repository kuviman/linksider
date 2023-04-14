use super::*;

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    LoadingLevel,
    Turn,
    WaitingForInput,
    Animation,
}
