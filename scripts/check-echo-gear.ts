import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import { Program, AnchorProvider, Wallet } from '@coral-xyz/anchor';
import * as fs from 'fs';
import * as path from 'path';

const WALLET = new PublicKey('bcNTWp8U8kDWrFkgwPjWxXE9WZf3iLeTmJx93PZgAQ8');
const SESSION_MANAGER_PROGRAM_ID = new PublicKey('6w1XVMSTRmZU9AWCKVvKohGAHSFMENhda7vqhKPQ8TPn');
const GAMEPLAY_STATE_PROGRAM_ID = new PublicKey('C8hK4qsqsSYQeqyXuTPTUUS3T7N74WnZCuzvChTpK1Mo');

const ITEM_NAMES: Record<string, string> = {
  'T-XX-00\0': 'Basic Pickaxe',
  'T-ST-01\0': 'Bulwark Shovel', 'T-ST-02\0': 'Granite Maul',
  'G-ST-01\0': 'Miner Helmet', 'G-ST-02\0': 'Work Vest',
  'G-ST-03\0': 'Iron Buckler', 'G-ST-04\0': 'Spiked Bracers',
  'G-ST-05\0': 'Stone Amulet', 'G-ST-06\0': 'Golem Core',
  'G-ST-07\0': 'Reinforcement Plate', 'G-ST-08\0': 'Mountain Heart',
  'T-SC-01\0': 'Scout Spade', 'T-SC-02\0': 'Whirlwind Pick',
  'G-SC-01\0': 'Swift Boots', 'G-SC-02\0': 'Pathfinder Cloak',
  'G-SC-03\0': 'Scout Goggles', 'G-SC-04\0': 'Wind Charm',
  'G-SC-05\0': 'Explorer Map', 'G-SC-06\0': 'Dash Emblem',
  'G-SC-07\0': 'Tempest Ring', 'G-SC-08\0': 'Phantom Steps',
  'T-GR-01\0': 'Fortune Pick', 'T-GR-02\0': 'Golden Drill',
  'G-GR-01\0': 'Lucky Pendant', 'G-GR-02\0': 'Coin Pouch',
  'G-GR-03\0': 'Treasure Sense', 'G-GR-04\0': 'Gilded Gloves',
  'G-GR-05\0': 'Midas Touch', 'G-GR-06\0': 'Hoard Ring',
  'G-GR-07\0': 'Dragon Vault', 'G-GR-08\0': 'Crown of Avarice',
  'T-BL-01\0': 'Blast Hammer', 'T-BL-02\0': 'Magma Drill',
  'G-BL-01\0': 'Dynamite Belt', 'G-BL-02\0': 'Powder Keg',
  'G-BL-03\0': 'Fuse Chain', 'G-BL-04\0': 'Bomb Satchel',
  'G-BL-05\0': 'Demolition Gear', 'G-BL-06\0': 'Inferno Core',
  'G-BL-07\0': 'Volcanic Mantle', 'G-BL-08\0': 'Cataclysm Engine',
  'T-FR-01\0': 'Rime Pike', 'T-FR-02\0': 'Glacier Auger',
  'G-FR-01\0': 'Frost Band', 'G-FR-02\0': 'Ice Shard Pendant',
  'G-FR-03\0': 'Snowdrift Cloak', 'G-FR-04\0': 'Frozen Gauntlet',
  'G-FR-05\0': 'Permafrost Shield', 'G-FR-06\0': 'Blizzard Horn',
  'G-FR-07\0': 'Avalanche Plate', 'G-FR-08\0': 'Deep Freeze Charm',
  'T-RU-01\0': 'Corrosion Pick', 'T-RU-02\0': 'Acid Bore',
  'G-RU-01\0': 'Rust Ring', 'G-RU-02\0': 'Tetanus Spike',
  'G-RU-03\0': 'Oxidizer Flask', 'G-RU-04\0': 'Decay Gauntlet',
  'G-RU-05\0': 'Blight Amulet', 'G-RU-06\0': 'Corrosive Shroud',
  'G-RU-07\0': 'Entropy Engine', 'G-RU-08\0': 'Annihilation Core',
  'T-BO-01\0': 'Blood Pick', 'T-BO-02\0': 'Leech Drill',
  'G-BO-01\0': 'Vitality Band', 'G-BO-02\0': 'Crimson Fang',
  'G-BO-03\0': 'Bloodstone', 'G-BO-04\0': 'Sanguine Gauntlet',
  'G-BO-05\0': 'Life Tap Amulet', 'G-BO-06\0': 'Hemoglobin Core',
  'G-BO-07\0': 'Vampiric Plate', 'G-BO-08\0': 'Crimson Eclipse',
  'T-TE-01\0': 'Tempo Mallet', 'T-TE-02\0': 'Rhythm Drill',
  'G-TE-01\0': 'Metronome Ring', 'G-TE-02\0': 'Beat Keeper',
  'G-TE-03\0': 'Sync Chain', 'G-TE-04\0': 'Cadence Bracer',
  'G-TE-05\0': 'Pulse Engine', 'G-TE-06\0': 'Harmony Amulet',
  'G-TE-07\0': 'Crescendo Plate', 'G-TE-08\0': 'Resonance Crown',
};

const TIER_NAMES = ['I', 'II', 'III'];
const OIL_FLAGS: Record<number, string> = { 0x01: 'ATK oil', 0x02: 'SPD oil', 0x04: 'DIG oil', 0x08: 'ARM oil' };

function decodeItemId(bytes: number[]): string { return String.fromCharCode(...bytes); }
function describeOils(flags: number): string {
  return Object.entries(OIL_FLAGS).filter(([bit]) => flags & Number(bit)).map(([,n]) => n).join(', ');
}

async function main() {
  for (const [label, rpc] of [['ER', 'http://127.0.0.1:7799'], ['Base', 'http://127.0.0.1:8899']]) {
    console.log(`\n--- ${label} (${rpc}) ---`);
    const connection = new Connection(rpc, 'processed');
    const [sessionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('gauntlet_session'), WALLET.toBuffer()], SESSION_MANAGER_PROGRAM_ID
    );
    const [gameStatePda] = PublicKey.findProgramAddressSync(
      [Buffer.from('game_state'), sessionPda.toBuffer()], GAMEPLAY_STATE_PROGRAM_ID
    );
    console.log('Session PDA:', sessionPda.toBase58());
    console.log('GameState PDA:', gameStatePda.toBase58());

    const acct = await connection.getAccountInfo(gameStatePda);
    if (!acct) { console.log('Not found on', label); continue; }

    const idl = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'target', 'idl', 'gameplay_state.json'), 'utf8'));
    const provider = new AnchorProvider(connection, new Wallet(Keypair.generate()), { commitment: 'processed' });
    const program = new Program(idl, provider);

    try {
      const gs = program.coder.accounts.decode('gameState', acct.data);
      console.log(`\nWeek: ${gs.week}, Phase: ${JSON.stringify(gs.phase)}, HP: ${gs.hp}`);

      for (let i = 0; i < gs.gauntletEchoes.length; i++) {
        const echo = gs.gauntletEchoes[i];
        if (!echo) { console.log(`\nEcho ${i+1}: (empty)`); continue; }
        const src = echo.source.bootstrap ? 'Bootstrap' : `Player(${echo.source.player?.toBase58()})`;
        console.log(`\n--- Echo ${i+1} (Week ${echo.week}) | ${src} | Gold: ${echo.loadout.goldAtBattleStart} ---`);
        if (echo.loadout.tool) {
          const id = decodeItemId(echo.loadout.tool.itemId);
          console.log(`  Tool: ${ITEM_NAMES[id] || id} (Tier ${TIER_NAMES[echo.loadout.tool.tier] || '?'}) ${describeOils(echo.loadout.tool.toolOilFlags)}`);
        }
        let count = 0;
        for (let g = 0; g < echo.loadout.gear.length; g++) {
          const gear = echo.loadout.gear[g];
          if (!gear) continue;
          count++;
          const id = decodeItemId(gear.itemId);
          console.log(`  Gear[${g}]: ${ITEM_NAMES[id] || id} (Tier ${TIER_NAMES[gear.tier] || '?'}) ${describeOils(gear.toolOilFlags)}`);
        }
        if (count === 0) console.log('  Gear: (none)');
        console.log(`  Total gear: ${count}/12 slots filled`);
      }
      return;
    } catch (err) { console.error('Decode failed:', err); }
  }
}
main().catch(console.error);
