//! UCI command parsing from stdin

use std::str::FromStr;

/// UCI commands that the engine can receive
#[derive(Debug, PartialEq, Clone)]
pub enum UciCommand {
    /// Initialize the UCI protocol
    Uci,
    /// Check if engine is ready
    IsReady,
    /// Set position from FEN or startpos with optional moves
    Position {
        fen: Option<String>,
        moves: Vec<String>,
    },
    /// Start searching with optional parameters
    Go {
        depth: Option<u8>,
        movetime: Option<u64>,
        infinite: bool,
    },
    /// Stop searching
    Stop,
    /// Quit the engine
    Quit,
    /// Set an option (UCI protocol feature, currently not implemented)
    SetOption { name: String, value: Option<String> },
    /// Unknown or unimplemented command
    Unknown(String),
}

impl FromStr for UciCommand {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(UciCommand::Unknown(String::new()));
        }

        let parts: Vec<&str> = s.split_whitespace().collect();
        let command = parts[0].to_lowercase();

        match command.as_str() {
            "uci" => Ok(UciCommand::Uci),
            "isready" => Ok(UciCommand::IsReady),
            "quit" => Ok(UciCommand::Quit),
            "stop" => Ok(UciCommand::Stop),

            "position" => parse_position_command(&parts[1..]),

            "go" => parse_go_command(&parts[1..]),

            "setoption" => parse_setoption_command(&parts[1..]),

            _ => Ok(UciCommand::Unknown(s.to_string())),
        }
    }
}

fn parse_position_command(parts: &[&str]) -> Result<UciCommand, String> {
    if parts.is_empty() {
        return Err("position command requires arguments".to_string());
    }

    let fen;
    let mut moves = Vec::new();
    let mut i = 0;

    // Parse position type (startpos or fen)
    if parts[i] == "startpos" {
        fen = None;
        i += 1;
    } else if parts[i] == "fen" {
        i += 1;
        // Collect FEN string (next 6 parts typically)
        let mut fen_parts = Vec::new();
        while i < parts.len() && parts[i] != "moves" {
            fen_parts.push(parts[i]);
            i += 1;
        }
        if fen_parts.is_empty() {
            return Err("fen requires position string".to_string());
        }
        fen = Some(fen_parts.join(" "));
    } else {
        return Err(format!("unknown position type: {}", parts[i]));
    }

    // Parse moves if present
    if i < parts.len() && parts[i] == "moves" {
        i += 1;
        while i < parts.len() {
            moves.push(parts[i].to_string());
            i += 1;
        }
    }

    Ok(UciCommand::Position { fen, moves })
}

fn parse_go_command(parts: &[&str]) -> Result<UciCommand, String> {
    let mut depth = None;
    let mut movetime = None;
    let mut infinite = false;
    let mut i = 0;

    while i < parts.len() {
        match parts[i] {
            "depth" => {
                i += 1;
                if i >= parts.len() {
                    return Err("depth requires a value".to_string());
                }
                depth = Some(
                    parts[i]
                        .parse::<u8>()
                        .map_err(|_| format!("invalid depth value: {}", parts[i]))?,
                );
                i += 1;
            }
            "movetime" => {
                i += 1;
                if i >= parts.len() {
                    return Err("movetime requires a value".to_string());
                }
                movetime = Some(
                    parts[i]
                        .parse::<u64>()
                        .map_err(|_| format!("invalid movetime value: {}", parts[i]))?,
                );
                i += 1;
            }
            "infinite" => {
                infinite = true;
                i += 1;
            }
            // Ignore other go parameters for now
            _ => {
                i += 1;
            }
        }
    }

    Ok(UciCommand::Go {
        depth,
        movetime,
        infinite,
    })
}

fn parse_setoption_command(parts: &[&str]) -> Result<UciCommand, String> {
    if parts.is_empty() || parts[0] != "name" {
        return Err("setoption requires 'name' parameter".to_string());
    }

    let mut name_parts = Vec::new();
    let mut value_parts = Vec::new();
    let mut in_value = false;
    let mut i = 1; // Skip "name"

    while i < parts.len() {
        if parts[i] == "value" {
            in_value = true;
            i += 1;
            continue;
        }

        if in_value {
            value_parts.push(parts[i]);
        } else {
            name_parts.push(parts[i]);
        }
        i += 1;
    }

    let name = name_parts.join(" ");
    let value = if value_parts.is_empty() {
        None
    } else {
        Some(value_parts.join(" "))
    };

    Ok(UciCommand::SetOption { name, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uci() {
        assert_eq!("uci".parse::<UciCommand>().unwrap(), UciCommand::Uci);
    }

    #[test]
    fn test_parse_isready() {
        assert_eq!(
            "isready".parse::<UciCommand>().unwrap(),
            UciCommand::IsReady
        );
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!("quit".parse::<UciCommand>().unwrap(), UciCommand::Quit);
    }

    #[test]
    fn test_parse_position_startpos() {
        let cmd = "position startpos".parse::<UciCommand>().unwrap();
        assert_eq!(
            cmd,
            UciCommand::Position {
                fen: None,
                moves: vec![]
            }
        );
    }

    #[test]
    fn test_parse_position_startpos_with_moves() {
        let cmd = "position startpos moves e2e4 e7e5"
            .parse::<UciCommand>()
            .unwrap();
        assert_eq!(
            cmd,
            UciCommand::Position {
                fen: None,
                moves: vec!["e2e4".to_string(), "e7e5".to_string()]
            }
        );
    }

    #[test]
    fn test_parse_position_fen() {
        let cmd = "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
            .parse::<UciCommand>()
            .unwrap();
        match cmd {
            UciCommand::Position {
                fen: Some(f),
                moves,
            } => {
                assert_eq!(
                    f,
                    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
                );
                assert_eq!(moves, Vec::<String>::new());
            }
            _ => panic!("Expected Position command with FEN"),
        }
    }

    #[test]
    fn test_parse_go_depth() {
        let cmd = "go depth 6".parse::<UciCommand>().unwrap();
        assert_eq!(
            cmd,
            UciCommand::Go {
                depth: Some(6),
                movetime: None,
                infinite: false
            }
        );
    }

    #[test]
    fn test_parse_go_movetime() {
        let cmd = "go movetime 1000".parse::<UciCommand>().unwrap();
        assert_eq!(
            cmd,
            UciCommand::Go {
                depth: None,
                movetime: Some(1000),
                infinite: false
            }
        );
    }

    #[test]
    fn test_parse_go_infinite() {
        let cmd = "go infinite".parse::<UciCommand>().unwrap();
        assert_eq!(
            cmd,
            UciCommand::Go {
                depth: None,
                movetime: None,
                infinite: true
            }
        );
    }

    #[test]
    fn test_parse_setoption() {
        let cmd = "setoption name Hash value 256"
            .parse::<UciCommand>()
            .unwrap();
        assert_eq!(
            cmd,
            UciCommand::SetOption {
                name: "Hash".to_string(),
                value: Some("256".to_string())
            }
        );
    }

    #[test]
    fn test_parse_unknown() {
        let cmd = "unknown command".parse::<UciCommand>().unwrap();
        assert_eq!(cmd, UciCommand::Unknown("unknown command".to_string()));
    }
}
