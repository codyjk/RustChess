use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Instant;

const DEFAULT_ELO: u32 = 1000;

pub struct Stockfish {
    process: Child,
    reader: BufReader<std::process::ChildStdout>,
    elo: u32,
}

impl Stockfish {
    pub fn new() -> Result<Self, std::io::Error> {
        let mut process = Command::new("stockfish")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let reader = BufReader::new(process.stdout.take().unwrap());

        Ok(Stockfish {
            process,
            reader,
            elo: DEFAULT_ELO,
        })
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
        self.send_command("setoption name UCI_LimitStrength value true")?;
        self.send_command(&format!("setoption name UCI_Elo value {}", elo))?;
        self.elo = elo;
        Ok(())
    }

    pub fn get_elo(&mut self) -> u32 {
        self.elo
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
