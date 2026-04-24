use arcis::prelude::*;

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

#[encrypted]
mod circuits {
    use arcis::prelude::*;
    use super::{AttackInput, AttackResult};

    #[instruction]
    pub fn resolve_attack(
        attack_input: Enc<Shared, AttackInput>,
    ) -> Enc<Shared, AttackResult> {
        let input = attack_input.to_arcis();

        let target_index: u8 = input.attack_y * 5 + input.attack_x;

        let mut hit: u8 = 0;
        let mut updated_board: [u8; 25] = input.defender_board;
        let mut i: u8 = 0;

        while i < 25 {
            let is_target: u8 = if i == target_index { 1 } else { 0 };
            hit = hit | (is_target & input.defender_board[i as usize]);
            updated_board[i as usize] = input.defender_board[i as usize]
                * (1 - (is_target & (input.defender_board[i as usize])));
            i = i + 1;
        }

        let mut defender_remaining: u8 = 0;
        let mut j: u8 = 0;
        while j < 25 {
            defender_remaining = defender_remaining + updated_board[j as usize];
            j = j + 1;
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
}
