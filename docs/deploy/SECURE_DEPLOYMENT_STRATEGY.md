# Secure Deployment Strategy (Devnet → Mainnet)

This document defines a safe, auditable deployment process that prevents loss of control, enables rapid upgrades initially, and transitions over time to decentralized governance with timelocks. It aligns with the program's emergency controls and authority model.

## Objectives
- Enable rapid iteration at launch without single-point-of-failure risk
- Prevent accidental or malicious loss of upgrade control
- Clean separation between deployer, upgrade authority, and in-program authority
- Rehearse everything on Devnet with the same topology and processes
- Transition to governance-controlled upgrades and parameters with timelocks

## Key Principles
- Separate keys per role: Deployer key (Program ID), Upgrade Authority, Runtime System Authority
- Never reuse Devnet keys on Mainnet
- Use Squads multisig for upgrade authority with clear interface and accurate pricing
- Keep hardware wallet as a signer in the multisig; do not use it directly for program deploy
- Record hashes, program IDs, authorities, and transactions in `deployment_info.json`
- Maintain a break-glass path (System Pause) independent from upgrade path

## Roles & Keys
- **Program ID Keypair** (aka deploy-keypair.json)
  - Purpose: Owns the immutable Program ID only. Used once during deployment.
  - File: `target/deploy/fixed_ratio_trading-keypair.json` (or devnet/mainnet variants)
  - Storage: Cold storage (paper/air-gapped). Never load to online hosts post-deploy.
- **Upgrade Authority** (Squads Multisig Address)
  - Purpose: Controls who can upgrade the program binary
  - Implementation: Squads multisig with clear UI and transparent pricing
  - Initially: Your keypair address (for deployment), then transferred to multisig address
  - Devnet/Mainnet: Start as multisig, can evolve to more complex governance later
  - Deployment pattern: Deploy with your EO keypair as upgrade authority, then run `set-upgrade-authority` to transfer to multisig
- **Runtime System Authority** (Hardware Wallet - davincij15)
  - Purpose: Controls pause/unpause, treasury withdrawals, fee changes, etc.
  - Implementation: Stored in PDA `SystemState.admin_authority`, initialized at launch
  - Devnet/Mainnet: `4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko` (davincij15 hardware wallet)
  - Note: Public-facing key for community recognition and trust
  - Hardware: Keystone hardware wallet (supports both networks)

## Environment Segregation
- Devnet and Mainnet use distinct:
  - Program ID keypairs
  - Upgrade authorities (different Squads multisigs)
  - Runtime authorities
  - Treasury PDAs (derive with explicit network discriminators)

## Multisig Setup Guide

### Prerequisites
- Solana wallet (Phantom, Solflare, etc.) with SOL for deployment fees
- Hardware wallet addresses for secure multisig membership
- List of trusted co-signer addresses

### Squads Multisig Setup
**Website:** https://app.squads.so/
**Documentation:** https://docs.squads.so/main/getting-started/create-a-squad

**Steps:**
1. Connect wallet and switch to desired network (Devnet/Mainnet)
2. Click "Create Squad"
3. Enter squad name and optional description/image
4. Add member public keys (up to 10 members)
5. Set confirmation threshold (e.g., 2-of-3, 3-of-5)
6. Review and deploy (~0.1 SOL cost)
7. Record the Squad address for use as upgrade authority

**Best Practices:**
- Include hardware wallet addresses as members
- Set threshold to prevent single points of failure (avoid 1-of-N)
- Test with small transactions before using for program upgrades

**Advantages over Realms:**
- Clear, user-friendly interface that matches documentation
- Transparent and accurate pricing
- Reliable multisig functionality
- Better user experience for upgrade operations

## Build & Supply Chain Hardening
- Build in a pinned container/VM: fixed Rust toolchain, Solana toolchain version, reproducible settings
- Produce and record:
  - SHA256 of the `fixed_ratio_trading.so`
  - Solana CLI version, rustc version, commit SHA, build timestamp
- Verify artifact hash pre- and post-deploy via `solana program dump` and local hash comparison
- Store all metadata and tx signatures in `deployment_info.json`

## Devnet Rehearsal (Mandatory)
1. Generate Devnet Program ID
   - `solana-keygen new --outfile target/deploy/devnet-fixed_ratio_trading-keypair.json`
2. Create Devnet Upgrade Authority (Squads Multisig)
   - Website: https://app.squads.so/
   - Switch wallet to Devnet, click "Create Squad"
   - Add member public keys (include your hardware wallet and trusted co-signers)
   - Set confirmation threshold (e.g., 2-of-3 for fast iteration)
   - Deploy Squad (~0.1 SOL cost), record the Squad address: `<DEVNET_UPGRADE_AUTH>`
3. Set Runtime System Authority
   - Initialize `SystemState.admin_authority = 4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko` (davincij15 hardware wallet)
   - Confirm admin operations validate against `SystemState` PDA and hardware wallet can sign
4. Build
   - `cargo build-bpf` (or your pinned build script)
   - Record artifact hash
5. Deploy (Devnet)
   - `solana program deploy target/deploy/fixed_ratio_trading.so \
      --program-id target/deploy/devnet-fixed_ratio_trading-keypair.json \
      --url https://api.devnet.solana.com \
      --upgrade-authority YOUR_KEYPAIR`
   - **CRITICAL**: Keep your keypair as upgrade authority for now (needed for initialization)
6. Verify Deployment
   - `solana program show <DEVNET_PROGRAM_ID>` → confirm your keypair is Upgrade Authority
   - Dump program and verify SHA256 matches built artifact
7. **CRITICAL: Initialize System State (MUST be done before authority transfer)**
   - Call `process_system_initialize` with your keypair as authority
   - This creates SystemState PDA with admin_authority = 4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko
   - Initialize Treasury PDAs and validate all system components
   - **WARNING**: System will be unusable if upgrade authority is transferred before initialization
8. Transfer Upgrade Authority (ONLY after successful initialization)
   - `solana program set-upgrade-authority <DEVNET_PROGRAM_ID> --new-upgrade-authority <DEVNET_UPGRADE_AUTH>`
   - Verify transfer: `solana program show <DEVNET_PROGRAM_ID>` → confirm Squad address is now upgrade authority
9. Exercise emergency controls
   - Pause/Unpause (system and per-pool)
   - Owner-only swaps toggling (ensure unified control per design)
10. Exercise Upgrade via Squads
   - Create upgrade proposal in Squads interface
   - Get required signatures from multisig members
   - Execute upgrade and verify success
   - Test rollback capabilities if needed
11. Run full e2e tests on Devnet against the deployed program
12. Capture all txids, slots, hashes in `deployment_info.json`
13. Disaster drill
   - Simulate key rotation of one signer in multisig
   - Verify continued ability to upgrade and operate pause

## Mainnet Launch Sequence
1. Preflight
   - Build in the same pinned environment
   - External binary diff vs tagged Devnet binary (source parity)
   - Independent sign-off on audit checklist
2. Generate Mainnet Program ID
   - `solana-keygen new --outfile target/deploy/mainnet-fixed_ratio_trading-keypair.json`
   - Cold-store the private key; never reused after deploy
3. Create Mainnet Upgrade Authority (Squads Multisig)
   - Website: https://app.squads.so/
   - Switch wallet to Mainnet, click "Create Squad"
   - Add member public keys (including hardware wallet and trusted co-signers)
   - Set confirmation threshold for secure operations (e.g., 3-of-5)
   - Deploy Squad (~0.1 SOL cost), record the Squad address: `<MAINNET_UPGRADE_AUTH>`
4. Set Runtime System Authority
   - Initialize/verify `SystemState.admin_authority = 4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko` (davincij15 hardware wallet)
5. Deploy (Mainnet)
   - `solana program deploy target/deploy/fixed_ratio_trading.so \
      --program-id target/deploy/mainnet-fixed_ratio_trading-keypair.json \
      --url https://api.mainnet-beta.solana.com \
      --upgrade-authority YOUR_KEYPAIR`
   - **CRITICAL**: Keep your keypair as upgrade authority for now (needed for initialization)
6. Verify Deployment
   - `solana program show <MAINNET_PROGRAM_ID>` → confirm your keypair is Upgrade Authority
   - Dump and hash-verify the on-chain binary
7. **CRITICAL: Initialize System State (MUST be done before authority transfer)**
   - Call `process_system_initialize` with your keypair as authority
   - This creates SystemState PDA with admin_authority = 4ekSqR4pNZ5hp4cRyicji1Yj7ZCphgkYQhwZf2ib9Wko
   - Initialize Treasury PDAs and validate all system components
   - **WARNING**: System will be unusable if upgrade authority is transferred before initialization
8. Transfer Upgrade Authority (ONLY after successful initialization)
   - `solana program set-upgrade-authority <MAINNET_PROGRAM_ID> --new-upgrade-authority <MAINNET_UPGRADE_AUTH>`
   - Verify transfer: `solana program show <MAINNET_PROGRAM_ID>` → confirm Squad address is now upgrade authority
9. Sanity Checks & Validation
   - Create a test pool with minimal funds; perform a test swap
   - Validate emergency pause/unpause with hardware wallet
10. Publish
   - Record final metadata and txids in `deployment_info.json`
   - Publish program ID, artifact hash, and a signed release manifest

## Rapid-Upgrade → Governance Transition
- Phase 1 (Weeks 0–2):
  - Upgrade authority: Squads multisig, threshold 2-of-3 (fast response)
  - Runtime authority: Hardware wallet (davincij15)
  - Implement a tested rollback buffer retained off-chain
- Phase 2 (Weeks 2–6):
  - Increase threshold (e.g., 3-of-5) for more security
  - Add more trusted members to the Squad
- Phase 3 (Long-term):
  - Consider migration to more advanced governance (if needed)
  - Keep emergency pause authority separate from upgrade authority
  - Maintain Squads for reliable multisig operations

### Changing Upgrade Authority (command)
- `solana program set-upgrade-authority <PROGRAM_ID> --new-upgrade-authority <NEW_AUTH>`
  - Execute via Squads interface with required multisig signatures

## Runtime Authority Configuration
- Preferred: PDA config (e.g., `SystemState`) storing `authority: Pubkey`
- Initialization: performed once by the Upgrade/Runtime authority signer
- Rotation: add instruction(s) allowing authority rotation gated by current authority
- Hardware wallet option: Admin operations signed directly by hardware wallet (current setup)

## Loss-of-Control Mitigations
- Never set Upgrade Authority to a single EO key on Mainnet
- Include hardware wallet(s) as signers inside the Squads multisig
- Maintain separate emergency pause authority (hardware wallet) that cannot upgrade or withdraw funds
- Regularly test:
  - Authority rotation within Squads
  - Pause/unpause operations
  - Upgrade and rollback via Squads interface
- Backups:
  - Program ID keypair (cold storage)
  - Squads multisig member rotation procedures
  - Out-of-band contact methods for co-signers

## Attestation & Record-Keeping
- Update `deployment_info.json` after each action with:
  - `network`, `programId`, `upgradeAuthority` (Squad address), `runtimeAuthority`
  - `binarySha256`, `solanaVersion`, `rustVersion`, `commitSha`
  - `deployTx`, `initTx`, `upgradeTx` (array), `rollbackTx` (array)
  - `timestamp`, `slot`
- Store a signed release manifest (PGP or hardware wallet signature over the metadata)

## Monitoring & Alerting (Post-Deploy)
- Watch program account and Squad multisig account for changes
- Alert on:
  - Any buffer write/upgrade proposal in Squads
  - System pause/unpause events
  - Authority rotation events
- Keep a public status page with current artifact hash and program ID

## Reference Commands
- Show program: `solana program show <PROGRAM_ID>`
- Dump program: `solana program dump <PROGRAM_ID> ./dumped.so && shasum -a 256 ./dumped.so`
- Set upgrade authority: `solana program set-upgrade-authority <PROGRAM_ID> --new-upgrade-authority <SQUAD_ADDRESS>`
- Deploy: `solana program deploy <PATH_TO_SO> --program-id <PATH_TO_KEYPAIR> --upgrade-authority <KEYPAIR_OR_SQUAD>`

---

This process lets you deploy quickly while retaining safety, then progressively decentralize control with clear recovery paths and auditable records using Squads' reliable multisig infrastructure.