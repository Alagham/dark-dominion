use arcis::prelude::*;

pub struct BoardInput {
    pub cells: [u8; 25],
    pub player_index: u8,
}

pub struct CommitBoardOutput {
    pub valid: u8,
    pub board_hash: u64,
    pub player_index: u8,
}

#[encrypted]
mod circuits {
    use arcis::prelude::*;
    use super::{BoardInput, CommitBoardOutput};

    #[instruction]
    pub fn commit_board(
        board_input: Enc<Shared, BoardInput>,
    ) -> Enc<Shared, CommitBoardOutput> {
        let input = board_input.to_arcis();

        let mut troop_count: u16 = 0;
        let mut i = 0;
        while i < 25 {
            troop_count = troop_count + (input.cells[i] as u16);
            i = i + 1;
        }

        let valid: u8 = if troop_count == 5 { 1 } else { 0 };

        let mut hash: u64 = 0;
        let prime: u64 = 31;
        let mut j = 0;
        while j < 25 {
            hash = hash.wrapping_mul(prime).wrapping_add(input.cells[j] as u64 + 1);
            j = j + 1;
        }

        let output = CommitBoardOutput {
            valid,
            board_hash: hash,
            player_index: input.player_index,
        };

        board_input.owner.from_arcis(output)
    }
}
