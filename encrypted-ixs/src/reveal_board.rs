use arcis::prelude::*;

pub struct RevealInput {
    pub original_board: [u8; 25],
    pub committed_hash: u64,
    pub player_index: u8,
}

pub struct RevealOutput {
    pub revealed_cells: [u8; 25],
    pub integrity_valid: u8,
    pub initial_troop_count: u8,
    pub player_index: u8,
}

#[encrypted]
mod circuits {
    use arcis::prelude::*;
    use super::{RevealInput, RevealOutput};

    #[instruction]
    pub fn reveal_board(
        reveal_input: Enc<Shared, RevealInput>,
    ) -> Enc<Shared, RevealOutput> {
        let input = reveal_input.to_arcis();

        let mut recomputed_hash: u64 = 0;
        let prime: u64 = 31;
        let mut i = 0;
        while i < 25 {
            recomputed_hash = recomputed_hash
                .wrapping_mul(prime)
                .wrapping_add(input.original_board[i] as u64 + 1);
            i = i + 1;
        }

        let integrity_valid: u8 = if recomputed_hash == input.committed_hash {
            1
        } else {
            0
        };

        let mut initial_troop_count: u8 = 0;
        let mut j = 0;
        while j < 25 {
            initial_troop_count = initial_troop_count + input.original_board[j];
            j = j + 1;
        }

        let output = RevealOutput {
            revealed_cells: input.original_board,
            integrity_valid,
            initial_troop_count,
            player_index: input.player_index,
        };

        reveal_input.owner.from_arcis(output)
    }
}
