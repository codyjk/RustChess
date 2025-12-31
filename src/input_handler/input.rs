//! Move input parsing and validation.

use std::str::FromStr;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

use crate::game::action::GameMode;

static COORD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("^([a-h][1-8])([a-h][1-8])$").expect("COORD_RE regex should be valid"));
static ALG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new("^([NBRQK]?[a-h]?[1-8]?x?[a-h][1-8](=[NBRQ])?[+#]?|O-O(-O)?)$")
        .expect("ALG_RE regex should be valid")
});

#[derive(Error, Debug)]
pub enum InputError {
    #[error("io error: {error:?}")]
    IOError { error: String },
    #[error("invalid input: {input:?}")]
    InvalidInput { input: String },
}

#[derive(Debug)]
pub enum MoveInput {
    Coordinate { from: String, to: String },
    Algebraic { notation: String },
    UseEngine,
}

#[derive(Debug)]
pub enum MenuInput {
    StartOver,
    Exit,
    SwitchGameMode { target: GameMode },
}

impl MenuInput {
    pub fn switch_to_play() -> Self {
        Self::SwitchGameMode {
            target: GameMode::Play,
        }
    }

    pub fn switch_to_watch() -> Self {
        Self::SwitchGameMode {
            target: GameMode::Watch,
        }
    }

    pub fn switch_to_pvp() -> Self {
        Self::SwitchGameMode {
            target: GameMode::Pvp,
        }
    }
}

impl FromStr for MenuInput {
    type Err = InputError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let trimmed = input.trim().to_lowercase();

        match trimmed.as_str() {
            "1" => Ok(MenuInput::StartOver),
            "q" => Ok(MenuInput::Exit),
            "2" => Ok(MenuInput::switch_to_play()),
            "3" => Ok(MenuInput::switch_to_watch()),
            "4" => Ok(MenuInput::switch_to_pvp()),
            _ => Err(InputError::InvalidInput {
                input: input.to_string(),
            }),
        }
    }
}

impl FromStr for MoveInput {
    type Err = InputError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = COORD_RE.captures(input) {
            return Ok(MoveInput::Coordinate {
                from: caps[1].to_string(),
                to: caps[2].to_string(),
            });
        }

        if let Some(caps) = ALG_RE.captures(input) {
            return Ok(MoveInput::Algebraic {
                notation: caps[1].to_string(),
            });
        }

        Err(InputError::InvalidInput {
            input: input.to_string(),
        })
    }
}

/// Parse chess move input (coordinates, algebraic notation, or "use engine")
/// Used during gameplay when entering moves
pub fn parse_move_input() -> Result<MoveInput, InputError> {
    use std::io::Write;

    let mut input = String::new();

    loop {
        if event::poll(std::time::Duration::from_millis(100)).map_err(|e| InputError::IOError {
            error: format!("Failed to poll event: {}", e),
        })? {
            if let Event::Key(KeyEvent { code, .. }) =
                event::read().map_err(|e| InputError::IOError {
                    error: format!("Failed to read event: {}", e),
                })?
            {
                match code {
                    KeyCode::Enter => {
                        if !input.is_empty() {
                            println!(); // Move to next line after input
                            break;
                        }
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        print!("{}", c); // Echo the character
                        std::io::stdout().flush().map_err(|e| InputError::IOError {
                            error: format!("Failed to flush stdout: {}", e),
                        })?;
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            print!("\x08 \x08"); // Erase character: backspace, space, backspace
                            std::io::stdout().flush().map_err(|e| InputError::IOError {
                                error: format!("Failed to flush stdout: {}", e),
                            })?;
                        }
                    }
                    KeyCode::Esc => {
                        // Allow Ctrl-C style exit
                        return Err(InputError::IOError {
                            error: "Input cancelled".to_string(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    input.trim().parse()
}

/// Parse menu input commands during game end
/// Handles immediate key presses for game control commands
pub fn parse_menu_input() -> Result<MenuInput, InputError> {
    loop {
        if event::poll(std::time::Duration::from_millis(100)).map_err(|e| InputError::IOError {
            error: format!("Failed to poll event: {}", e),
        })? {
            if let Event::Key(KeyEvent { code, .. }) =
                event::read().map_err(|e| InputError::IOError {
                    error: format!("Failed to read event: {}", e),
                })?
            {
                match code {
                    KeyCode::Char('1') => return Ok(MenuInput::StartOver),
                    KeyCode::Char('q') => return Ok(MenuInput::Exit),
                    KeyCode::Char('2') => return Ok(MenuInput::switch_to_play()),
                    KeyCode::Char('3') => return Ok(MenuInput::switch_to_watch()),
                    KeyCode::Char('4') => return Ok(MenuInput::switch_to_pvp()),
                    _ => {} // Ignore other keys
                }
            }
        }
    }
}
