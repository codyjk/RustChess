use crate::board::square;

pub const EMPTY: u64 = 0x0000000000000000;
pub const ALL: u64 = 0xFFFFFFFFFFFFFFFF;

pub const A_FILE: u64 = 0x0101010101010101;
pub const B_FILE: u64 = A_FILE << 1;
pub const C_FILE: u64 = A_FILE << 2;
pub const D_FILE: u64 = A_FILE << 3;
pub const E_FILE: u64 = A_FILE << 4;
pub const F_FILE: u64 = A_FILE << 5;
pub const G_FILE: u64 = A_FILE << 6;
pub const H_FILE: u64 = A_FILE << 7;

pub const RANK_1: u64 = 0xFF;
pub const RANK_2: u64 = RANK_1 << (8 * 1);
pub const RANK_3: u64 = RANK_1 << (8 * 2);
pub const RANK_4: u64 = RANK_1 << (8 * 3);
pub const RANK_5: u64 = RANK_1 << (8 * 4);
pub const RANK_6: u64 = RANK_1 << (8 * 5);
pub const RANK_7: u64 = RANK_1 << (8 * 6);
pub const RANK_8: u64 = RANK_1 << (8 * 7);

pub fn render_occupied(targets: u64) -> String {
    let divider = "+---+---+---+---+---+---+---+---+";
    let files: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
    let ranks: [char; 8] = ['1', '2', '3', '4', '5', '6', '7', '8'];

    let mut rows: Vec<String> = vec![];

    for rank in (0..8).rev() {
        let mut cells: Vec<String> = vec![];
        for file in 0..8 {
            let sq = square::from_rank_file(rank, file);
            let cell = match sq & targets {
                0 => ' ',
                _ => 'X',
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

    rows.join("\n")
}
