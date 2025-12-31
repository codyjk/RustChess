//! Game action results for mode switching

/// Mode identifier for tracking current game mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Play,
    Watch,
    Pvp,
}

impl GameMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "play" => Some(GameMode::Play),
            "watch" => Some(GameMode::Watch),
            "pvp" => Some(GameMode::Pvp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameAction {
    /// Restart the game in the same mode
    RestartSameMode,
    /// Switch to a different game mode
    SwitchGameMode { target: GameMode },
    /// Exit the application
    Exit,
}
