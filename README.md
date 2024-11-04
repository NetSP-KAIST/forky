# Fork State-Aware Differential Fuzzing for Blockchain Consensus Implementations
## Forky Proof-of-Concept Implementation
Repo for anonymous submission and artifact evaluation

**Open only in the review period** (to be reopened as a public with identity after review).

**(Warning)** Current PoC implementation utilizes a large amount of disk I/O that can impact SSD lifespan.

### Forky Bitcoin
(**forky-bitcoin**) Differential testing Bitcoin fork resolution implementation with Forky
* Target client: Bitcoin Core (C++), Bitcoin Knots (C++), btcd (Go), bcoin (JavaScript)

### Forky Ethereum 1.0 (PoW) 
(**forky-eth1**) Fuzzer for testing Ethereum 1.0 (PoW) fork resolution implementation with Forky
* Target client: Geth (Go)

### Forky Ethereum 2.0 (PoS)
(**forky-eth2**) Differential testing Ethereum 2.0 fork resolution implementation with Forky
* Target client: Prysm (Go), Lighthouse (Rust), Teku (Java), Nimbus (Nim)
