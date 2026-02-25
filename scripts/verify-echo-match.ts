import { Connection, PublicKey, Keypair } from '@solana/web3.js';
import { Program, AnchorProvider, Wallet } from '@coral-xyz/anchor';
import * as fs from 'fs';
import * as path from 'path';

const WALLET = new PublicKey('bcNTWp8U8kDWrFkgwPjWxXE9WZf3iLeTmJx93PZgAQ8');
const SESSION_MANAGER_PROGRAM_ID = new PublicKey('6w1XVMSTRmZU9AWCKVvKohGAHSFMENhda7vqhKPQ8TPn');
const GAMEPLAY_STATE_PROGRAM_ID = new PublicKey('C8hK4qsqsSYQeqyXuTPTUUS3T7N74WnZCuzvChTpK1Mo');

function itemIdStr(bytes: number[]): string {
  return String.fromCharCode(...bytes).replace(/\0/g, '');
}

async function main() {
  const erConn = new Connection('http://127.0.0.1:7799', 'processed');
  const baseConn = new Connection('http://127.0.0.1:8899', 'processed');
  const idl = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'target', 'idl', 'gameplay_state.json'), 'utf8'));
  const erProvider = new AnchorProvider(erConn, new Wallet(Keypair.generate()), { commitment: 'processed' });
  const erProgram = new Program(idl, erProvider);
  const baseProvider = new AnchorProvider(baseConn, new Wallet(Keypair.generate()), { commitment: 'processed' });
  const baseProgram = new Program(idl, baseProvider);

  const [sessionPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('gauntlet_session'), WALLET.toBuffer()], SESSION_MANAGER_PROGRAM_ID
  );
  const [gameStatePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('game_state'), sessionPda.toBuffer()], GAMEPLAY_STATE_PROGRAM_ID
  );

  // Get game state echo 0
  const gsAcct = await erConn.getAccountInfo(gameStatePda);
  if (!gsAcct) { console.log('GameState not found'); return; }
  const gs = erProgram.coder.accounts.decode('gameState', gsAcct.data);
  const echo0 = gs.gauntletEchoes[0];
  const echo0Tool = echo0?.loadout?.tool ? itemIdStr(echo0.loadout.tool.itemId) : null;
  const echo0Gear = echo0?.loadout?.gear?.filter((g: any) => g)?.map((g: any) => itemIdStr(g.itemId)) ?? [];
  console.log('=== GameState Echo 0 ===');
  console.log('Tool:', echo0Tool);
  console.log('Gear:', echo0Gear);

  // Get ALL week 1 pool entries
  const [weekPoolPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('gauntlet_week_pool'), Buffer.from([1])], GAMEPLAY_STATE_PROGRAM_ID
  );

  for (const [label, program] of [['Base', baseProgram], ['ER', erProgram]] as const) {
    try {
      const weekPool = await (program.account as any).gauntletWeekPool.fetch(weekPoolPda);
      const entries = weekPool.entries as any[];
      console.log(`\n=== [${label}] Week 1 Pool (${entries.length} entries) ===`);

      for (let i = 0; i < entries.length; i++) {
        const e = entries[i];
        const tool = e.loadout.tool ? itemIdStr(e.loadout.tool.itemId) : '(none)';
        const gear = e.loadout.gear.filter((g: any) => g).map((g: any) => itemIdStr(g.itemId));
        const matchesGameState = tool === echo0Tool &&
          gear.length === echo0Gear.length &&
          gear.every((id: string, idx: number) => id === echo0Gear[idx]);
        console.log(`  [${i}] Tool: ${tool} | Gear(${gear.length}): ${gear.join(', ')}${matchesGameState ? ' *** MATCHES GAME STATE ***' : ''}`);
      }
    } catch (err: any) {
      console.log(`[${label}] Failed:`, err.message?.slice(0, 80));
    }
  }
}
main().catch(console.error);
