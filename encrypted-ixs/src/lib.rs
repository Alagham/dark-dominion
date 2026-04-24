use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    pub struct BoardInput {
        pub cells: [u8; 25],
        pub player_index: u8,
    }

    pub struct CommitBoardOutput {
        pub valid: u8,
        pub board_hash: u64,
        pub player_index: u8,
    }

    pub struct AttackInput {
        pub attack_x: u8,
        pub attack_y: u8,
        pub defender_index: u8,
        pub defender_board: [u8; 25],
    }

    pub struct AttackResult {
        pub hit: u8,
        pub defender_remaining: u8,
        pub attacked_x: u8,
        pub attacked_y: u8,
        pub updated_board: [u8; 25],
    }

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

    #[instruction]
    pub fn commit_board(
        board_input: Enc<Shared, BoardInput>,
    ) -> Enc<Shared, CommitBoardOutput> {
        let input = board_input.to_arcis();
        let mut troop_count: u16 = 0;
        for i in 0..25_usize {
            troop_count = troop_count + (input.cells[i] as u16);
        }
        let valid: u8 = if troop_count == 5 { 1 } else { 0 };
        let mut hash: u64 = 0;
        let prime: u64 = 31;
        for j in 0..25_usize {
            hash = hash.wrapping_mul(prime).wrapping_add(input.cells[j] as u64 + 1);
        }
        let output = CommitBoardOutput {
            valid,
            board_hash: hash,
            player_index: input.player_index,
        };
        board_input.owner.from_arcis(output)
    }

    #[instruction]
    pub fn resolve_attack(
        attack_input: Enc<Shared, AttackInput>,
    ) -> Enc<Shared, AttackResult> {
        let input = attack_input.to_arcis();
        let target_index: u8 = input.attack_y * 5 + input.attack_x;
        let mut hit: u8 = 0;
        let mut updated_board: [u8; 25] = input.defender_board;
        for i in 0..25_usize {
            let is_target: u8 = if i as u8 == target_index { 1 } else { 0 };
            let cell = input.defender_board[i];
            let was_hit = is_target * cell;
            hit = if was_hit == 1 { 1 } else { hit };
            updated_board[i] = cell * (1 - was_hit);
        }
        let mut defender_remaining: u8 = 0;
        for j in 0..25_usize {
            defender_remaining = defender_remaining + updated_board[j];
        }
        let result = AttackResult {
            hit,
            defender_remaining,
            attacked_x: input.attack_x,
            attacked_y: input.attack_y,
            updated_board,
        };
        attack_input.owner.from_arcis(result)
    }

    #[instruction]
    pub fn reveal_board(
        reveal_input: Enc<Shared, RevealInput>,
    ) -> Enc<Shared, RevealOutput> {
        let input = reveal_input.to_arcis();
        let mut recomputed_hash: u64 = 0;
        let prime: u64 = 31;
        for i in 0..25_usize {
            recomputed_hash = recomputed_hash
                .wrapping_mul(prime)
                .wrapping_add(input.original_board[i] as u64 + 1);
        }
        let integrity_valid: u8 = if recomputed_hash == input.committed_hash {
            1
        } else {
            0
        };
        let mut initial_troop_count: u8 = 0;
        for j in 0..25_usize {
            initial_troop_count = initial_troop_count + input.original_board[j];
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
