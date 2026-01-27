import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PoiSystem } from "../../target/types/poi_system";
import { expect } from "chai";
import { Keypair, SystemProgram } from "@solana/web3.js";

describe("poi-system", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PoiSystem as Program<PoiSystem>;
  const sessionManagerProgramId = new anchor.web3.PublicKey(
    "FcMT7MzBLVQGaMATEMws3fjsL2Q77QSHmoEPdowTMxJa",
  );

  const getMapPoisPDA = (sessionPubkey: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_pois"), sessionPubkey.toBuffer()],
      program.programId,
    );
  };

  const createSessionAccount = async (): Promise<Keypair> => {
    const session: Keypair = Keypair.generate();
    const rent = await provider.connection.getMinimumBalanceForRentExemption(0);
    const transaction = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: provider.wallet.publicKey,
        newAccountPubkey: session.publicKey,
        lamports: rent,
        space: 0,
        programId: sessionManagerProgramId,
      }),
    );

    await provider.sendAndConfirm(transaction, [session]);
    return session;
  };

  it("rejects non-session account", async () => {
    const session = provider.wallet.publicKey;
    const [mapPoisPDA] = getMapPoisPDA(session);

    try {
      await program.methods
        .initializeMapPois(1, 1, new anchor.BN(42))
        .accounts({
          mapPois: mapPoisPDA,
          session,
          payer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
      expect.fail("Should have thrown InvalidSessionOwner error");
    } catch (error: any) {
      expect(error.toString()).to.include("InvalidSessionOwner");
    }
  });

  it("initializes map pois for session account", async () => {
    const session = await createSessionAccount();
    const [mapPoisPDA] = getMapPoisPDA(session.publicKey);

    await program.methods
      .initializeMapPois(1, 1, new anchor.BN(42))
      .accounts({
        mapPois: mapPoisPDA,
        session: session.publicKey,
        payer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    const mapPois = await program.account.mapPois.fetch(mapPoisPDA);
    expect(mapPois.session.toString()).to.equal(session.publicKey.toString());
  });
});
