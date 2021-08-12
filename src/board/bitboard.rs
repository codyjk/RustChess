use crate::board::square;

pub type Bitboard = u64;

pub const EMPTY: Bitboard = 0x0000000000000000;
pub const ALL: Bitboard = 0xFFFFFFFFFFFFFFFF;

pub const A_FILE: Bitboard = 0x0101010101010101;
pub const B_FILE: Bitboard = A_FILE << 1;
pub const C_FILE: Bitboard = A_FILE << 2;
pub const D_FILE: Bitboard = A_FILE << 3;
pub const E_FILE: Bitboard = A_FILE << 4;
pub const F_FILE: Bitboard = A_FILE << 5;
pub const G_FILE: Bitboard = A_FILE << 6;
pub const H_FILE: Bitboard = A_FILE << 7;

pub const RANK_1: Bitboard = 0xFF;
pub const RANK_2: Bitboard = RANK_1 << (8 * 1);
pub const RANK_3: Bitboard = RANK_1 << (8 * 2);
pub const RANK_4: Bitboard = RANK_1 << (8 * 3);
pub const RANK_5: Bitboard = RANK_1 << (8 * 4);
pub const RANK_6: Bitboard = RANK_1 << (8 * 5);
pub const RANK_7: Bitboard = RANK_1 << (8 * 6);
pub const RANK_8: Bitboard = RANK_1 << (8 * 7);

pub fn render_occupied(targets: u64) -> String {
    let divider = "+---+---+---+---+---+---+---+---+";
    let files: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
    let ranks: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

    let mut rows: Vec<String> = vec![];

    let row_iter = (0..8).rev();
    let col_iter = 0..8;

    for row in row_iter {
        let mut cells: Vec<String> = vec![];
        for col in col_iter.clone() {
            let sq = square::from_row_col(row, col);
            let cell = match sq & targets {
                0 => ' ',
                _ => 'X',
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
