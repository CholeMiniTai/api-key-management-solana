# Superteam Earn Submission Draft — API Key Management System (Solana + Anchor)

## Project links
- GitHub: https://github.com/CholeMiniTai/api-key-management-solana
- Program language/framework: Rust + Anchor
- Network: Solana Devnet

## What I built
I rebuilt a production-style backend pattern (**API Key Management with RBAC**) as an on-chain Solana program.

Core lifecycle supported on-chain:
1. initialize_registry
2. create_api_key
3. revoke_api_key
4. rotate_api_key
5. update_permissions
6. record_usage
7. verify_api_key
8. close_api_key

## Why this design
Most submissions tend to implement rate-limiters. I chose **API Key Management** because it requires richer account modeling and state transitions:
- deterministic PDAs per owner/key
- permission bitmask authorization
- expiry + active state validity checks
- immutable, verifiable lifecycle transitions via transaction history

## Production perspective (Web2 → Solana)
- On-chain is source of truth for ownership/revocation/permissions.
- Off-chain cache is recommended for low-latency request paths.
- Hybrid model gives strong integrity + acceptable UX at scale.

## Validation evidence
- `anchor test` passing
  - program unit tests: 11/11
  - integration tests: 5/5
- CLI implemented for all 8 instruction operations + list/show
- Devnet proof links (all key actions covered):
  - deploy: https://explorer.solana.com/tx/4x36fxCcYqDDEQ1kTq2VWXYpdpc5mDupr3246qVUH5VdSZ9Bwu1dkYNv11oG4ZvURLFWNCKx6TGw5XJTgkWBFRC2?cluster=devnet
  - initialize: https://explorer.solana.com/tx/KeV5iinENfV6j1vNNgmQcwfW9uUHwjSPCBZUAMvCSUceZcmPPkDXTwpAyXNE5fcNuvN8S34tXcgqtd12GM3GyDu?cluster=devnet
  - create: https://explorer.solana.com/tx/4Xf4SgxaMEAs4zqFaoY2JF4doXjthwAk3749zYhyzJCp1fqLfrTm4gfNvkGXesV25V2wbbTFhjeAkPGCeGCtgZr9?cluster=devnet
  - verify: https://explorer.solana.com/tx/3Ch4KuAnx5Sjtqw1FRCHagGSSrs4jHCMkzC45wGTDcFKjJApFpCJow9e5fpSwf9s3nUwcQFVq8YRmWjysLesBY1X?cluster=devnet
  - rotate: https://explorer.solana.com/tx/36SbL4bAy3g7rMSFUQWgUhea6o3aD7UR1mSK5KsyVeVwbKUjHxefyNVMJJqjiKrqqrA4SHczowu8dvtU9eMpeK9v?cluster=devnet
  - update permissions: https://explorer.solana.com/tx/DRN8m4AgVsW1F8NhgAeaPBvk1xvC2ysYgKp9s9KybC3AcTaphpHgtqDJZArk9T1rQAzihRk8Fng9SAaAEHdNNN9?cluster=devnet
  - record usage: https://explorer.solana.com/tx/HjkWJKGgPRCY2nxRqafukYk42ykdcZuwEYDeZEb8wzvqfj1mPWJjCTqLfqwwwEvLChbDUzL2TNAKathvV3pEFL3?cluster=devnet
  - revoke: https://explorer.solana.com/tx/2sr55MW6hEZcyvAPfPU1uTA2CxNpLpb5nWZRotWEgT2P6fze6iwCwexz4acy4N1bejYPQs1FrBF8ya6GjzvkvjbH?cluster=devnet
  - close: https://explorer.solana.com/tx/32JYaDQzZiKXsvsoDYYyQ8xtv5JWcf5k7amCdbCER6NepqAwYn9KDLZygisfQ1PxqqZa2wAdwutUHcepEij3uK6d?cluster=devnet

## Notes for judges
README includes:
- Web2 vs Solana 8-dimension comparison
- account layout diagram
- tradeoff analysis including measured rent
- requirement-to-evidence mapping
