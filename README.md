# 🏰 Dark Dominion — Encrypted Strategy War Game

> Fully onchain hidden-information strategy game powered by Arcium MPC on Solana

**Program ID:** 6Byt42WoRsHCeSXTY7Rov118FryQRGsZqcJQqupYR1SW (Solana Devnet)

**Live Demo:** http://localhost:3000

## What is Dark Dominion?

Dark Dominion is a 2-player hidden-information strategy war game on Solana. Players secretly deploy 5 troops on a 5x5 grid and take turns attacking. Neither player can see the opponents board. Arcium MPC resolves every attack with zero information leakage.

## How Arcium Powers This

- commit_board circuit: Validates 5 troops placed, stores encrypted board in MXE. Neither player sees opponent board.
- resolve_attack circuit: Computes hit/miss on encrypted data, returns only 1 bit. Board never revealed.
- reveal_board circuit: End-game integrity proof, verifies no cheating occurred.

## Why This Is Unique

Traditional onchain games are fully transparent. ZK-proofs only prove statements. Arcium MXE is the only technology that lets two players share encrypted game state that no single node can see — enabling true fog-of-war onchain.

## Project Structure

- encrypted-ixs/src/lib.rs — Arcium MPC circuits (Arcis)
- programs/dark_dominion/ — Solana Anchor program
- app-frontend/ — Next.js game UI
- tests/ — Integration tests

## Quick Start

yarn install
arcium build
anchor program deploy --provider.cluster devnet
cd app-frontend && npm run dev

## License
MIT
