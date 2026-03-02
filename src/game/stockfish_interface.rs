use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

pub struct Stockfish {
    process: Child,
    reader: BufReader<std::process::ChildStdout>,
    elo: u32,
    min_elo: u32,
}

impl Stockfish {
    pub fn new() -> Result<Self, std::io::Error> {
        let mut process = Command::new("stockfish")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let reader = BufReader::new(process.stdout.take().unwrap());

        let mut sf = Stockfish {
            process,
            reader,
            elo: 0,
            min_elo: 1320, // Stockfish default, updated by init()
        };
        sf.init()?;
        Ok(sf)
    }

    /// Send "uci" and parse option lines to discover the minimum UCI_Elo.
    fn init(&mut self) -> Result<(), std::io::Error> {
        self.send_command("uci")?;
        loop {
            let line = self.read_line()?;
            // Parse: option name UCI_Elo type spin default 1320 min 1320 max 3190
            if line.starts_with("option name UCI_Elo") {
                if let Some(min_val) = line
                    .split_whitespace()
                    .skip_while(|&w| w != "min")
                    .nth(1)
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    self.min_elo = min_val;
                }
            }
            if line == "uciok" {
                break;
            }
        }
        Ok(())
    }

    /// Returns the minimum ELO that Stockfish supports.
    pub fn min_elo(&self) -> u32 {
        self.min_elo
    }

    pub fn send_command(&mut self, command: &str) -> Result<(), std::io::Error> {
        writeln!(self.process.stdin.as_mut().unwrap(), "{}", command)?;
        Ok(())
    }

    pub fn read_line(&mut self) -> Result<String, std::io::Error> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(line.trim().to_string())
    }

    pub fn set_elo(&mut self, elo: u32) -> Result<(), std::io::Error> {
        let clamped = elo.max(self.min_elo);
        self.send_command("setoption name UCI_LimitStrength value true")?;
        self.send_command(&format!("setoption name UCI_Elo value {}", clamped))?;
        // Ensure Stockfish has applied the options before we start a game
        self.send_command("isready")?;
        loop {
            let line = self.read_line()?;
            if line == "readyok" {
                break;
            }
        }
        self.elo = clamped;
        Ok(())
    }

    pub fn get_best_move(
        &mut self,
        position: &str,
        time_limit: u64,
    ) -> Result<(String, u64), std::io::Error> {
        self.send_command(&format!("position startpos moves {}", position))?;
        self.send_command(&format!("go movetime {}", time_limit))?;

        let start_time = Instant::now();
        let best_move;

        loop {
            let line = self.read_line()?;
            if line.starts_with("bestmove") {
                best_move = line.split_whitespace().nth(1).unwrap().to_string();
                break;
            }
        }

        let elapsed_time = start_time.elapsed().as_millis() as u64;
        Ok((best_move, elapsed_time))
    }
}

impl Drop for Stockfish {
    fn drop(&mut self) {
        let _ = self.send_command("quit");
    }
}
