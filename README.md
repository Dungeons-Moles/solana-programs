# Dungeons & Moles Solana Programs

This workspace contains three Anchor programs that power the on-chain gameplay systems for Dungeons & Moles.

## Programs

- `player-profile`: Player identity, tier unlocks, run tracking
- `session-manager`: Session lifecycle for gameplay (MagicBlock delegation stubbed for now)
- `map-generator`: Deterministic seed configuration for map generation

## PDA Derivations

Use the following PDA seeds for client integration.

### Player Profile Program

- `PlayerProfile`: `"player" + owner_pubkey`
  - Seeds: `[b"player", owner.key().as_ref()]`
- `Treasury`: `"treasury"`
  - Seeds: `[b"treasury"]`

### Session Manager Program

- `GameSession`: `"session" + player_pubkey`
  - Seeds: `[b"session", player.key().as_ref()]`
- `SessionCounter`: `"session_counter"`
  - Seeds: `[b"session_counter"]`

### Map Generator Program

- `MapConfig`: `"map_config"`
  - Seeds: `[b"map_config"]`

## Build & Test

```bash
anchor build
anchor test
```

## Notes

- MagicBlock SDK integration is currently stubbed in-program due to Solana toolchain compatibility.
- Map generation happens off-chain; on-chain verification compares hashes.
