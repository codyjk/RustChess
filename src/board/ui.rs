use super::square::Square;
use super::Board;

impl Board {
    pub fn to_ascii(&self) -> String {
        let divider = "+---+---+---+---+---+---+---+---+";
        let files: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
        let ranks: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

        let mut rows: Vec<String> = vec![];

        let row_iter = (0..8).rev();
        let col_iter = 0..8;

        for row in row_iter {
            let mut cells: Vec<String> = vec![];
            for col in col_iter.clone() {
                let sq = Square::from_row_col(row, col);
                let cell = match self.get(sq) {
                    Some((piece, color)) => piece.to_fen(color),
                    None => ' ',
                };
                cells.push(cell.to_string());
            }
            let formatted_cells = format!("| {} |", cells.join(" | "));

            rows.push(format!("{} {}", ' ', divider));
            rows.push(format!("{} {}", ranks[row], formatted_cells));
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

        rows.join("\n")
    }
}
