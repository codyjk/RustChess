use super::square::Square;
use super::Board;

impl Board {
    pub fn to_ascii(&self) -> String {
        let divider = "+---+---+---+---+---+---+---+---+\n";
        let rows: Vec<String> = (0..=7)
            .rev()
            .map(|row| {
                let cells: Vec<String> = (0..=7)
                    .map(|col| {
                        let square = Square::from_row_col(row, col);
                        match self.get(square) {
                            Some((piece, color)) => piece.to_fen(color),
                            None => ' ',
                        }
                    })
                    .map(|ch| ch.to_string())
                    .collect();
                format!("| {} |\n", cells.join(" | "))
            })
            .collect();
        format!("{}{}{}", divider, rows.join(divider), divider)
    }
}
