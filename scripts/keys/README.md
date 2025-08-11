Key Management and Safe Storage Locations

All private keys have been removed from the repository. Do not commit any key material. This document defines where keys are stored locally for each Solana environment and how scripts reference them.

Recommended directory structure

- Localnet
  - Upgrade Authority: `~/.FRTKeys/localnet/upgrade-authority-keypair.json`
  - Operator Wallet: `~/.FRTKeys/localnet/wallet-keypair.json`
  - Program ID (optional): `~/.FRTKeys/localnet/program-id-keypair.json`

- Devnet
  - Upgrade Authority: `~/.FRTKeys/devnet/upgrade-authority-keypair.json`
  - Operator Wallet: `~/.FRTKeys/devnet/wallet-keypair.json`
  - Program ID (optional): `~/.FRTKeys/devnet/program-id-keypair.json`

- Mainnet
  - Upgrade Authority: `~/.FRTKeys/mainnet/upgrade-authority-keypair.json`
  - Operator Wallet: `~/.FRTKeys/mainnet/wallet-keypair.json`
  - Program ID (optional): `~/.FRTKeys/mainnet/program-id-keypair.json`

Notes

- Keys are not part of the repo; paths above are examples for local secure storage.
- Set permissions to owner-only: `chmod 600 <keypair.json>`
- Back up keys securely; rotate immediately if exposure is suspected.

How scripts find keys

- General (Solana CLI default): `~/.config/solana/id.json`
  - Many scripts default to this path unless overridden.

- `scripts/remote_build_and_deploy.sh`
  - Uses `KEYPAIR_PATH` (defaults to `~/.config/solana/id.json`).
  - Override:
    - One-time: `KEYPAIR_PATH=~/.FRTKeys/devnet/upgrade-authority-keypair.json ./scripts/remote_build_and_deploy.sh`
    - Shell: `export KEYPAIR_PATH=~/.FRTKeys/devnet/upgrade-authority-keypair.json`

- `scripts/initialize_system.js`
  - Usage: `node scripts/initialize_system.js <PROGRAM_ID> [RPC_URL] [KEYPAIR_PATH]`
  - Default `KEYPAIR_PATH`: `~/.config/solana/id.json`
  - Example: `node scripts/initialize_system.js <PROGRAM_ID> http://localhost:8899 ~/.FRTKeys/localnet/upgrade-authority-keypair.json`

Program ID keypair

- If you pin a program ID (via `--program-id`), store that keypair under the matching environment (e.g., `~/.FRTKeys/devnet/program-id-keypair.json`).
- Otherwise, Solana will generate a program ID keypair in the build/deploy artifacts as needed.

Security reminders

- Never commit key files to git or share them in logs.
- Avoid embedding private keys in scripts or docs; always reference files via paths or environment variables.
- Consider using a secure secrets manager for production deployments.
