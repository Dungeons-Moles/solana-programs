use anchor_lang::prelude::*;

pub mod archetypes;
pub mod constants;
pub mod errors;
pub mod spawner;
pub mod state;
pub mod traits;

pub use errors::FieldEnemiesError;
pub use state::*;

declare_id!("4cyqGxGRHBb1gR73ssCa53Hapv7UmmETsXYEC9Keg1PR");

/// Map dimensions for enemy placement
const MAP_WIDTH: u8 = 32;
const MAP_HEIGHT: u8 = 32;
const MAP_TILES: usize = (MAP_WIDTH as usize) * (MAP_HEIGHT as usize);

/// Simple linear congruential generator for deterministic pseudo-random values
/// Uses standard LCG parameters (multiplier and increment from Numerical Recipes)
#[inline]
fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

#[program]
pub mod field_enemies {
    use super::*;

    /// Initialize map enemies for a session
    ///
    /// Spawns enemies using biome-weighted archetype selection and tier distribution.
    /// Enemy positions are distributed across the map grid.
    pub fn initialize_map_enemies(
        ctx: Context<InitializeMapEnemies>,
        act: u8,
        level: u8,
    ) -> Result<()> {
        require!(act >= 1 && act <= 4, FieldEnemiesError::InvalidAct);

        let map_enemies = &mut ctx.accounts.map_enemies;
        map_enemies.session = ctx.accounts.session.key();
        map_enemies.bump = ctx.bumps.map_enemies;

        // Get spawn count for this act
        let spawn_count = spawner::spawn_count_for_act(act);

        // Generate seed from session key and level for deterministic randomness
        let session_bytes = ctx.accounts.session.key().to_bytes();
        let mut seed: u64 = level as u64;
        for (i, byte) in session_bytes.iter().enumerate().take(8) {
            seed = seed.wrapping_add((*byte as u64) << (i * 8));
        }

        // Spawn enemies
        let mut enemies = Vec::with_capacity(spawn_count as usize);
        let mut occupied = [false; MAP_TILES];
        occupied[0] = true;

        for i in 0..spawn_count {
            // Generate pseudo-random values from seed
            seed = lcg_next(seed);
            let tier_rand = (seed >> 8) as u8;
            seed = lcg_next(seed);
            let arch_rand = (seed >> 8) as u8;
            seed = lcg_next(seed);
            let pos_rand = seed;

            // Sample tier and archetype
            let tier = spawner::sample_tier(tier_rand, act);
            let archetype_id = spawner::sample_archetype(arch_rand, act);

            // Calculate position - distribute enemies across the map grid
            let mut idx = ((pos_rand as usize).wrapping_add(i as usize)) % MAP_TILES;
            if idx == 0 {
                idx = 1;
            }
            let mut final_x = 1u8;
            let mut final_y = 1u8;

            for _ in 0..MAP_TILES {
                let x = (idx % MAP_WIDTH as usize) as u8;
                let y = (idx / MAP_WIDTH as usize) as u8;
                if !occupied[idx] {
                    occupied[idx] = true;
                    final_x = x;
                    final_y = y;
                    break;
                }
                idx = (idx + 1) % MAP_TILES;
                if idx == 0 {
                    idx = 1;
                }
            }

            enemies.push(EnemyInstance {
                archetype_id,
                tier: tier as u8,
                x: final_x,
                y: final_y,
                defeated: false,
            });
        }

        map_enemies.enemies = enemies;
        map_enemies.count = spawn_count;

        emit!(EnemiesSpawned {
            session: map_enemies.session,
            count: map_enemies.count,
            act,
        });

        Ok(())
    }

    /// Mark an enemy as defeated after combat victory
    pub fn mark_enemy_defeated(ctx: Context<MarkEnemyDefeated>, x: u8, y: u8) -> Result<u8> {
        let map_enemies = &mut ctx.accounts.map_enemies;
        let session = map_enemies.session;

        // Find and update enemy
        let (archetype_id, tier, gold_reward) = {
            let enemy = map_enemies
                .get_enemy_at_position_mut(x, y)
                .ok_or(FieldEnemiesError::EnemyNotFound)?;

            require!(!enemy.defeated, FieldEnemiesError::EnemyAlreadyDefeated);

            enemy.defeated = true;
            let tier_enum = EnemyTier::from_u8(enemy.tier).unwrap_or_default();
            (enemy.archetype_id, enemy.tier, tier_enum.gold_reward())
        };

        emit!(EnemyDefeated {
            session,
            archetype_id,
            tier,
            x,
            y,
            gold_reward,
        });

        Ok(gold_reward)
    }
}

#[derive(Accounts)]
pub struct InitializeMapEnemies<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Session account validated by caller
    pub session: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + MapEnemies::INIT_SPACE,
        seeds = [MapEnemies::SEED_PREFIX, session.key().as_ref()],
        bump
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MarkEnemyDefeated<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub map_enemies: Account<'info, MapEnemies>,
}

#[event]
pub struct EnemiesSpawned {
    pub session: Pubkey,
    pub count: u8,
    pub act: u8,
}

#[event]
pub struct EnemyDefeated {
    pub session: Pubkey,
    pub archetype_id: u8,
    pub tier: u8,
    pub x: u8,
    pub y: u8,
    pub gold_reward: u8,
}
