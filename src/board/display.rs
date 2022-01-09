use super::square;
use super::Board;
use std::fmt;

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let divider = "+---+---+---+---+---+---+---+---+";
        let files: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
        let ranks: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

        let mut rows: Vec<String> = vec![];

        for rank in (0..8).rev() {
            let mut cells: Vec<String> = vec![];
            for file in 0..8 {
                let sq = square::from_rank_file(rank, file);
                let cell = match self.get(sq) {
                    Some((piece, color)) => piece.to_fen(color),
                    None => ' ',
                };
                cells.push(cell.to_string());
            }
            let formatted_cells = format!("| {} |", cells.join(" | "));

            rows.push(format!("{} {}", ' ', divider));
            rows.push(format!("{} {}", ranks[rank as usize], formatted_cells));
        }
        rows.push(format!("{} {}", ' ', divider));
        let formatted_ranks_footer = format!(
            "  {}  ",
            files
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join("   ")
        );
        rows.push(format!("{} {}", ' ', formatted_ranks_footer));

        write!(f, "{}", rows.join("\n"))
    }
}
