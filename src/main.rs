use chess::board::color::Color;
use chess::game::command::MakeRandomMove;
use chess::game::Game;
use chess::input_handler;
use termion::clear;

fn main() {
    let game = &mut Game::new();
    let player_color = Color::White;
    loop {
        println!("{}", clear::All);
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
            Box::new(MakeRandomMove {})
        };

        match command.execute(game) {
            Ok(()) => {
                game.next_turn();
                continue;
            }
            Err(error) => println!("error: {}", error),
        }
    }
}
