<!--
Sync Impact Report
==================
Version change: 0.0.0 → 1.0.0
Added sections:
  - I. Security-First
  - II. Test-Driven Development
  - III. Program Composability
  - IV. Anchor Framework
  - V. MagicBlock Ephemeral Rollups
  - Security & Verification Standards
  - Development Workflow
  - Governance
Templates requiring updates:
  - .specify/templates/plan-template.md ✅ (no changes needed - generic)
  - .specify/templates/spec-template.md ✅ (no changes needed - generic)
  - .specify/templates/tasks-template.md ✅ (no changes needed - generic)
Follow-up TODOs: None
-->

# Solana Programs Constitution

## Core Principles

### I. Security-First

All Solana programs MUST be developed with security as the primary concern. This is non-negotiable for on-chain code handling user assets.

- All programs MUST pass security review before deployment
- All account validation MUST use Anchor's constraint system (`#[account(...)]`)
- All arithmetic MUST use checked operations or explicitly handle overflow
- All PDAs MUST have deterministic, collision-resistant seeds
- Signer and ownership checks MUST be explicit and documented
- No unsafe Rust operations in program code
- All external CPI calls MUST validate the target program ID

### II. Test-Driven Development

Tests MUST be written before implementation. Red-Green-Refactor cycle is strictly enforced.

- Tests MUST be written and fail before implementation begins
- Unit tests MUST cover all instruction handlers
- Integration tests MUST cover all user flows using Bankrun or similar local validator
- Test coverage MUST reach minimum 80% for all programs
- Tests MUST include edge cases: invalid accounts, unauthorized signers, arithmetic boundaries
- Fuzz testing SHOULD be applied to complex logic

### III. Program Composability

Programs MUST be designed for composability with the broader Solana ecosystem.

- All programs MUST expose a clean IDL via Anchor
- CPIs (Cross-Program Invocations) MUST use typed CpiContext where possible
- Account structures MUST be documented with clear serialization format
- Programs MUST NOT assume exclusive ownership of shared resources
- Event emission MUST follow standard patterns for indexer compatibility

### IV. Anchor Framework

Anchor MUST be used for all program development to ensure type safety and security.

- All programs MUST use Anchor framework (latest stable version)
- Account validation MUST use Anchor constraints, not manual checks
- Error handling MUST use custom Anchor error enums with descriptive messages
- IDL MUST be generated and version-controlled
- Anchor's `declare_id!` MUST match deployed program addresses

### V. MagicBlock Ephemeral Rollups

Programs MUST support MagicBlock's Ephemeral Rollup infrastructure for high-performance gaming sessions.

- Game session programs MUST be compatible with ephemeral rollup delegation
- State MUST be designed for efficient delegation and undelegation
- Programs MUST handle the ephemeral → permanent state commit pattern
- Session-scoped state MUST be clearly separated from persistent state
- Programs MUST validate delegation authority correctly

## Security & Verification Standards

Programs handling critical game logic or player assets MUST meet enhanced verification standards.

- Critical arithmetic logic SHOULD be formally verifiable where tooling permits
- All programs MUST pass `cargo clippy` with no warnings
- All programs MUST pass `cargo audit` with no known vulnerabilities
- Upgrade authority MUST be documented and follow multisig or DAO governance
- State migration paths MUST be defined before any upgrade
- Breaking changes MUST include migration instructions and backward compatibility period

## Development Workflow

All development MUST follow a structured workflow ensuring quality and traceability.

- All changes MUST go through pull request review
- PRs MUST include test coverage for new functionality
- PRs MUST pass all CI checks: build, test, lint, audit
- Documentation MUST be updated alongside code changes
- All instructions MUST have IDL documentation describing purpose, accounts, and behavior
- Usage examples MUST be provided for complex instructions

## Governance

This constitution supersedes all other development practices for the Solana programs repository.

- Amendments require documented rationale, team review, and migration plan
- All PRs and code reviews MUST verify compliance with these principles
- Exceptions MUST be documented in code with explicit justification
- Version changes follow semantic versioning: MAJOR for breaking changes, MINOR for new features, PATCH for fixes
- Runtime development guidance is maintained in `CLAUDE.md` at repository root

**Version**: 1.0.0 | **Ratified**: 2025-01-15 | **Last Amended**: 2025-01-15
