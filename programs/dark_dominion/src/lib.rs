use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_COMMIT_BOARD: u32 = comp_def_offset("commit_board");
const COMP_DEF_OFFSET_RESOLVE_ATTACK: u32 = comp_def_offset("resolve_attack");
const COMP_DEF_OFFSET_REVEAL_BOARD: u32 = comp_def_offset("reveal_board");

declare_id!("6Byt42WoRsHCeSXTY7Rov118FryQRGsZqcJQqupYR1SW");

#[arcium_program]
pub mod dark_dominion {
    use super::*;

    pub fn init_commit_board_comp_def(ctx: Context<InitCommitBoardCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn init_resolve_attack_comp_def(ctx: Context<InitResolveAttackCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn init_reveal_board_comp_def(ctx: Context<InitRevealBoardCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn create_game(ctx: Context<CreateGame>, game_id: u32) -> Result<()> {
        let game = &mut ctx.accounts.game;
        game.game_id = game_id;
        game.player_a = ctx.accounts.creator.key();
        game.player_b = Pubkey::default();
        game.state = GameState::WaitingForPlayerB;
        game.turn = 0;
        game.player_a_troops_remaining = 5;
        game.player_b_troops_remaining = 5;
        game.player_a_board_hash = 0;
        game.player_b_board_hash = 0;
        game.player_a_committed = false;
        game.player_b_committed = false;
        game.winner = Pubkey::default();
        game.created_at = Clock::get()?.unix_timestamp;
        emit!(GameCreatedEvent {
            game_id,
            player_a: ctx.accounts.creator.key(),
        });
        Ok(())
    }

    pub fn join_game(ctx: Context<JoinGame>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        require!(game.state == GameState::WaitingForPlayerB, DarkDominionError::GameNotOpen);
        require!(game.player_a != ctx.accounts.player_b.key(), DarkDominionError::CannotPlayYourself);
        game.player_b = ctx.accounts.player_b.key();
        game.state = GameState::WaitingForCommits;
        emit!(PlayerJoinedEvent {
            game_id: game.game_id,
            player_b: ctx.accounts.player_b.key(),
        });
        Ok(())
    }

    pub fn commit_board(
        ctx: Context<CommitBoard>,
        computation_offset: u64,
        cell_ciphertexts: [[u8; 32]; 25],
        pub_key: [u8; 32],
        nonce: u128,
        player_index: u8,
    ) -> Result<()> {
        let game = &ctx.accounts.game;
        require!(
            ctx.accounts.player.key() == game.player_a || ctx.accounts.player.key() == game.player_b,
            DarkDominionError::NotAPlayer
        );
        require!(game.state == GameState::WaitingForCommits, DarkDominionError::WrongGameState);
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let mut args = ArgBuilder::new()
            .x25519_pubkey(pub_key)
            .plaintext_u128(nonce)
            .plaintext_u8(player_index);
        for ct in &cell_ciphertexts {
            args = args.encrypted_u8(*ct);
        }
        let args = args.build();
        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CommitBoardCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "commit_board")]
    pub fn commit_board_callback(
        ctx: Context<CommitBoardCallback>,
        output: SignedComputationOutputs<CommitBoardOutput>,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let result = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(r) => r,
            Err(_) => return Err(DarkDominionError::AbortedComputation.into()),
        };
        // Arcium auto-generates field_0, field_1, field_2 for each output field
        let valid = result.field_0.ciphertexts[0][0];
        require!(valid == 1, DarkDominionError::InvalidBoard);
        let hash_bytes: [u8; 8] = result.field_0.ciphertexts[1][..8].try_into().unwrap();
        let hash_val = u64::from_le_bytes(hash_bytes);
        let p_idx = result.field_0.ciphertexts[2][0];
        if p_idx == 0 {
            game.player_a_board_hash = hash_val;
            game.player_a_committed = true;
        } else {
            game.player_b_board_hash = hash_val;
            game.player_b_committed = true;
        }
        if game.player_a_committed && game.player_b_committed {
            game.state = GameState::InProgress;
            game.turn = 0;
        }
        emit!(BoardCommittedEvent {
            game_id: game.game_id,
            player_index: p_idx,
            board_hash: hash_val,
        });
        Ok(())
    }

    pub fn attack(
        ctx: Context<Attack>,
        computation_offset: u64,
        attack_x: u8,
        attack_y: u8,
        defender_board_ciphertexts: [[u8; 32]; 25],
        pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let game = &ctx.accounts.game;
        require!(game.state == GameState::InProgress, DarkDominionError::WrongGameState);
        let is_player_a_turn = game.turn % 2 == 0;
        if is_player_a_turn {
            require!(ctx.accounts.attacker.key() == game.player_a, DarkDominionError::NotYourTurn);
        } else {
            require!(ctx.accounts.attacker.key() == game.player_b, DarkDominionError::NotYourTurn);
        }
        require!(attack_x < 5 && attack_y < 5, DarkDominionError::InvalidCoordinate);
        let defender_index: u8 = if is_player_a_turn { 1 } else { 0 };
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let mut args = ArgBuilder::new()
            .x25519_pubkey(pub_key)
            .plaintext_u128(nonce)
            .plaintext_u8(attack_x)
            .plaintext_u8(attack_y)
            .plaintext_u8(defender_index);
        for ct in &defender_board_ciphertexts {
            args = args.encrypted_u8(*ct);
        }
        let args = args.build();
        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![ResolveAttackCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "resolve_attack")]
    pub fn resolve_attack_callback(
        ctx: Context<ResolveAttackCallback>,
        output: SignedComputationOutputs<ResolveAttackOutput>,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let result = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(r) => r,
            Err(_) => return Err(DarkDominionError::AbortedComputation.into()),
        };
        let hit_ct = result.field_0.ciphertexts[0];
        let remaining_ct = result.field_0.ciphertexts[1];
        let attacked_x = result.field_0.ciphertexts[2][0];
        let attacked_y = result.field_0.ciphertexts[3][0];
        let attacking_a = game.turn % 2 == 0;
        game.turn += 1;
        game.last_attack_x = attacked_x;
        game.last_attack_y = attacked_y;
        game.last_hit_ciphertext = hit_ct;
        game.last_remaining_ciphertext = remaining_ct;
        emit!(AttackResolvedEvent {
            game_id: game.game_id,
            attacker: if attacking_a { game.player_a } else { game.player_b },
            attacked_x,
            attacked_y,
            hit_ciphertext: hit_ct,
            remaining_ciphertext: remaining_ct,
            turn: game.turn - 1,
        });
        Ok(())
    }

    pub fn declare_victory(ctx: Context<DeclareVictory>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        require!(game.state == GameState::InProgress, DarkDominionError::WrongGameState);
        require!(
            ctx.accounts.claimant.key() == game.player_a || ctx.accounts.claimant.key() == game.player_b,
            DarkDominionError::NotAPlayer
        );
        game.state = GameState::Finished;
        game.winner = ctx.accounts.claimant.key();
        emit!(GameEndedEvent {
            game_id: game.game_id,
            winner: ctx.accounts.claimant.key(),
            total_turns: game.turn,
        });
        Ok(())
    }

    pub fn reveal_board(
        ctx: Context<RevealBoard>,
        computation_offset: u64,
        original_board_ciphertexts: [[u8; 32]; 25],
        committed_hash: u64,
        player_index: u8,
        pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let game = &ctx.accounts.game;
        require!(game.state == GameState::Finished, DarkDominionError::WrongGameState);
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let mut args = ArgBuilder::new()
            .x25519_pubkey(pub_key)
            .plaintext_u128(nonce)
            .plaintext_u64(committed_hash)
            .plaintext_u8(player_index);
        for ct in &original_board_ciphertexts {
            args = args.encrypted_u8(*ct);
        }
        let args = args.build();
        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![RevealBoardCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "reveal_board")]
    pub fn reveal_board_callback(
        ctx: Context<RevealBoardCallback>,
        output: SignedComputationOutputs<RevealBoardOutput>,
    ) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let result = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(r) => r,
            Err(_) => return Err(DarkDominionError::AbortedComputation.into()),
        };
        let integrity = result.field_0.ciphertexts[0][0];
        let troop_count = result.field_0.ciphertexts[1][0];
        let player_index = result.field_0.ciphertexts[2][0];
        if player_index == 0 {
            game.player_a_integrity_valid = integrity == 1;
        } else {
            game.player_b_integrity_valid = integrity == 1;
        }
        emit!(BoardRevealedEvent {
            game_id: game.game_id,
            player_index,
            integrity_valid: integrity,
            initial_troop_count: troop_count,
        });
        Ok(())
    }
}

#[account]
#[derive(Default)]
pub struct GameAccount {
    pub game_id: u32,
    pub player_a: Pubkey,
    pub player_b: Pubkey,
    pub state: GameState,
    pub turn: u32,
    pub player_a_troops_remaining: u8,
    pub player_b_troops_remaining: u8,
    pub player_a_board_hash: u64,
    pub player_b_board_hash: u64,
    pub player_a_committed: bool,
    pub player_b_committed: bool,
    pub player_a_integrity_valid: bool,
    pub player_b_integrity_valid: bool,
    pub winner: Pubkey,
    pub last_attack_x: u8,
    pub last_attack_y: u8,
    pub last_hit_ciphertext: [u8; 32],
    pub last_remaining_ciphertext: [u8; 32],
    pub created_at: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Default)]
pub enum GameState {
    #[default]
    WaitingForPlayerB,
    WaitingForCommits,
    InProgress,
    Finished,
}

#[queue_computation_accounts("commit_board", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct CommitBoard<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub player: Signer<'info>,
    #[account(mut)]
    pub game: Account<'info, GameAccount>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_COMMIT_BOARD))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("commit_board")]
#[derive(Accounts)]
pub struct CommitBoardCallback<'info> {
    #[account(mut)]
    pub game: Account<'info, GameAccount>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_COMMIT_BOARD))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: checked by arcium
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions sysvar
    pub instructions_sysvar: AccountInfo<'info>,
}

#[queue_computation_accounts("resolve_attack", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct Attack<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub attacker: Signer<'info>,
    #[account(mut)]
    pub game: Account<'info, GameAccount>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_RESOLVE_ATTACK))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("resolve_attack")]
#[derive(Accounts)]
pub struct ResolveAttackCallback<'info> {
    #[account(mut)]
    pub game: Account<'info, GameAccount>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_RESOLVE_ATTACK))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: checked by arcium
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions sysvar
    pub instructions_sysvar: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct DeclareVictory<'info> {
    #[account(mut)]
    pub claimant: Signer<'info>,
    #[account(mut)]
    pub game: Account<'info, GameAccount>,
}

#[queue_computation_accounts("reveal_board", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct RevealBoard<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub player: Signer<'info>,
    #[account(mut)]
    pub game: Account<'info, GameAccount>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, DarkDominionError::ClusterNotSet))]
    /// CHECK: checked by arcium
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_REVEAL_BOARD))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("reveal_board")]
#[derive(Accounts)]
pub struct RevealBoardCallback<'info> {
    #[account(mut)]
    pub game: Account<'info, GameAccount>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_REVEAL_BOARD))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: checked by arcium
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, DarkDominionError::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions sysvar
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("commit_board", payer)]
#[derive(Accounts)]
pub struct InitCommitBoardCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: not initialized yet
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address lookup table
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut program
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("resolve_attack", payer)]
#[derive(Accounts)]
pub struct InitResolveAttackCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: not initialized yet
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address lookup table
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut program
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("reveal_board", payer)]
#[derive(Accounts)]
pub struct InitRevealBoardCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: not initialized yet
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address lookup table
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut program
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(game_id: u32)]
pub struct CreateGame<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        space = 8 + 300,
        seeds = [b"game", &game_id.to_le_bytes()],
        bump
    )]
    pub game: Account<'info, GameAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinGame<'info> {
    #[account(mut)]
    pub player_b: Signer<'info>,
    #[account(mut)]
    pub game: Account<'info, GameAccount>,
}

#[event]
pub struct GameCreatedEvent {
    pub game_id: u32,
    pub player_a: Pubkey,
}

#[event]
pub struct PlayerJoinedEvent {
    pub game_id: u32,
    pub player_b: Pubkey,
}

#[event]
pub struct BoardCommittedEvent {
    pub game_id: u32,
    pub player_index: u8,
    pub board_hash: u64,
}

#[event]
pub struct AttackResolvedEvent {
    pub game_id: u32,
    pub attacker: Pubkey,
    pub attacked_x: u8,
    pub attacked_y: u8,
    pub hit_ciphertext: [u8; 32],
    pub remaining_ciphertext: [u8; 32],
    pub turn: u32,
}

#[event]
pub struct GameEndedEvent {
    pub game_id: u32,
    pub winner: Pubkey,
    pub total_turns: u32,
}

#[event]
pub struct BoardRevealedEvent {
    pub game_id: u32,
    pub player_index: u8,
    pub integrity_valid: u8,
    pub initial_troop_count: u8,
}

#[error_code]
pub enum DarkDominionError {
    #[msg("Game is not open")]
    GameNotOpen,
    #[msg("Cannot play yourself")]
    CannotPlayYourself,
    #[msg("Not a player in this game")]
    NotAPlayer,
    #[msg("Wrong game state")]
    WrongGameState,
    #[msg("Invalid board: place exactly 5 troops")]
    InvalidBoard,
    #[msg("Not your turn")]
    NotYourTurn,
    #[msg("Invalid coordinate")]
    InvalidCoordinate,
    #[msg("Computation aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
}



pub type ErrorCode = DarkDominionError;
