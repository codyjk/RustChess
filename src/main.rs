use chess::board::color::Color;
use chess::game::command::MakeShallowOptimalMove;
use chess::game::Game;
use chess::input_handler;
use termion::clear;

fn main() {
    let game = &mut Game::new();
    let player_color = Color::White;
    println!("{}", clear::All);
    loop {
        println!("{}", game.render_board());
        let command = if player_color == game.turn() {
            match input_handler::parse_command() {
                Ok(command) => command,
                Err(msg) => {
                    println!("{}", msg);
                    continue;
                }
            }
        } else {
            Box::new(MakeShallowOptimalMove {})
        };

        match command.execute(game) {
            Ok(chessmove) => {
                println!("{}", clear::All);
                println!("computer chose {} for {}", chessmove, game.turn());
                game.next_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
