# Stream-Scholar Contracts

A comprehensive educational platform built on Stellar/Soroban blockchain featuring course streaming, scholarship management, and dynamic Student Profile NFTs.

## Overview

Stream-Scholar is a decentralized learning platform that combines traditional educational features with modern blockchain technology. The platform includes:

- **Course Streaming**: Pay-per-minute educational content streaming
- **Scholarship System**: On-chain scholarship funding and management
- **Student Profile NFTs**: Dynamic NFTs that grow with student achievements
- **Governance**: Admin and global course veto mechanisms
- **Gas Optimization**: Smart gas estimation and subsidy features

## Quick Start

### Prerequisites

- Rust 1.70+ (for Soroban contracts)
- Node.js 16+ (for frontend and NFT features)
- Docker and Docker Compose (for local testing)
- Stellar account (testnet for development)

### Local Test Network Setup

To set up a local test network with Docker that pre-loads 5 dummy courses and 100 test USDC:

1. Ensure Docker and Docker Compose are installed.

2. Run the following command in the project root:
   ```bash
   docker compose up
   ```

3. The setup script will:
   - Start a local Soroban network
   - Generate and fund test accounts (admin, teacher, student)
   - Deploy a USDC token contract and mint 100 USDC to the student account
   - Deploy the scholar contract and initialize it
   - Add 5 dummy courses to the registry

4. The network will remain running. The contract IDs and account details will be displayed in the output.

### Testing the Setup

To verify the setup is successful:

1. In a new terminal, exec into the running container:
   ```bash
   docker compose exec soroban-local bash
   ```

2. Check the list of courses:
   ```bash
   soroban contract invoke --id <SCHOLAR_CONTRACT_ID> --network standalone -- list_courses
   ```
   Expected output: `[1,2,3,4,5]`

3. Check the USDC balance of the student:
   ```bash
   soroban contract invoke --id <USDC_TOKEN_ID> --network standalone -- balance --id <STUDENT_ADDRESS>
   ```
   Expected output: `1000000000` (100 USDC with 7 decimals)

## Student Profile NFT Feature

### Overview

The Student Profile NFT system transforms a student's learning journey into a unique, tradable non-fungible token on the Stellar blockchain. Each NFT represents a student's profile that dynamically evolves based on their educational achievements.

### Key Features

- **Dynamic Leveling**: NFTs automatically level up (1-8) based on accumulated XP
- **Achievement System**: Unlock badges and achievements that add visual flair to your NFT
- **Course Tracking**: Link completed courses to earn XP and level up
- **Study Streaks**: Maintain daily study habits for bonus rewards
- **Visual Progression**: NFT artwork changes based on level and achievements
- **Stellar Native**: Built on Stellar for low fees and fast transactions
- **Transferable**: Own, trade, or gift your learning profile NFT

### NFT Installation

```bash
# Install NFT dependencies
npm install

# Environment Setup
cp .env.example .env
# Edit .env with your Stellar credentials

# Deploy NFT contract
npm run deploy

# Mint a student profile NFT
npm run mint student123 "Alice Johnson" alice@example.com

# Run frontend
npm run dev
```

### Level System

| Level | Name | Required XP | Visual Theme |
|-------|------|-------------|--------------|
| 1 | Beginner | 0 | Gray 🌱 |
| 2 | Novice | 100 | Silver 📖 |
| 3 | Apprentice | 250 | Bronze ⚒️ |
| 4 | Scholar | 500 | Gold 🎓 |
| 5 | Expert | 1,000 | Emerald 💎 |
| 6 | Master | 2,000 | Blue 👑 |
| 7 | Grandmaster | 5,000 | Purple 🔮 |
| 8 | Legend | 10,000 | Orange 🏆 |

## Project Structure

```text
.
├── contracts/
│   ├── scholar_contracts/     # Main Soroban contracts
│   └── token/                 # USDC token contract
├── src/                       # NFT Student Profile system
├── frontend/                  # NFT web interface
├── scripts/                   # Deployment and management scripts
├── docs/                      # Documentation
├── tests/                     # Test files
└── docker-compose.yml         # Local network setup
```

## Core Features

### Course Streaming
- Pay-per-minute streaming model
- Session management and validation
- Dynamic pricing based on demand
- Teacher revenue sharing

### Scholarship System
- On-chain scholarship funding
- Teacher-restricted withdrawals
- Scholarship role management
- Transparent fund allocation

### Governance
- Admin course veto and revocation
- Global course veto mechanism
- Platform governance features
- Community-driven decisions

### Gas Optimization
- Gas estimation service
- Subsidy mechanisms for students
- Optimized contract interactions
- Cost-effective streaming

### Security & Validation
- **Comprehensive string validation** preventing empty/malicious inputs
- **XSS and injection protection** for all metadata fields
- **Input sanitization** with character filtering and length limits
- **Structured error handling** with descriptive error codes (600-612)
- **Security hold mechanisms** for institutional oversights

## Development

### Building Contracts

```bash
# Build Soroban contracts
cd contracts/scholar_contracts
cargo build --target wasm32-unknown-unknown --release

# Run tests
cargo test
```

### Frontend Development

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build
```

### Testing

```bash
# Run contract tests
cargo test

# Run NFT tests
npm test

# Run integration tests
npm run test:integration
```

## Documentation

- [Instructor Onboarding Guide](docs/INSTRUCTOR_ONBOARDING_GUIDE.md)
- [WASM Size Benchmarking](docs/WASM_SIZE_BENCHMARKING.md)
- [Course Metadata Implementation](docs/course-metadata-implementation-guide.md)
- [Contribution Guidelines](CONTRIBUTING.md)

## Security Features

### Session Management
The platform implements advanced session management to prevent unauthorized access:

It natively extends the existing `heartbeat` function to validate a unique 32-byte `session_hash` (passed via the previously unused `_signature` parameter), ensuring complete backward compatibility with zero breaking changes to the API.

**How it works:**
* **Accepted Session:** When a heartbeat is received, it checks the stored session hash. If the hash matches the active session, or if the previous session has safely timed out (exceeding the `heartbeat_interval`), the stream is securely permitted.
* **Rejected Session:** If the incoming hash does not match the stored hash *and* the previous session is currently active, the contract explicitly rejects the heartbeat. This immediately halts unauthorized parallel streams or duplicate logins.

### Access Control
- Role-based permissions
- Teacher authentication
- Student withdrawal protections
- Admin governance controls

## Network Deployment

### Testnet Deployment

```bash
# Deploy to testnet
./scripts/deploy.sh testnet

# Verify deployment
soroban contract info --id <CONTRACT_ID> --network testnet
```

### Mainnet Deployment

```bash
# Deploy to mainnet
./scripts/deploy.sh mainnet

# Note: Ensure sufficient gas fees and proper configuration
```

## Analytics & Monitoring

- WASM size benchmarking
- Gas usage optimization
- Performance metrics
- User engagement tracking

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## Roadmap

### Phase 1 - Core Features 
- [x] Course streaming platform
- [x] Scholarship system
- [x] Governance mechanisms
- [x] Gas optimization
- [x] Student Profile NFTs

### Phase 2 - Enhanced Features (In Progress)
- [ ] Social features (following, leaderboards)
- [ ] Course marketplace integration
- [ ] Advanced analytics dashboard
- [ ] Mobile app development

### Phase 3 - Ecosystem (Future)
- [ ] DAO governance for achievement standards
- [ ] Cross-chain compatibility
- [ ] Institutional partnerships
- [ ] Scholarship programs

---

**Built with ❤️ by the Stream-Scholar Team**

*Transforming education into verifiable digital achievements on the Stellar blockchain.*

## 💰 Bounty Contribution

- **Task:** Veto Powers for Tier-1 Sponsors
- **Reward:** $1
- **Source:** GitHub-Paid
- **Date:** 2026-04-27

