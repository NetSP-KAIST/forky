# Forky
[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.14641017.svg)](https://doi.org/10.5281/zenodo.14641017)

This is the GitHub repository for ICSE'25 paper [[paper link]](https://doi.ieeecomputersociety.org/10.1109/ICSE55347.2025.00085): 

> *Fork State-Aware Differential Fuzzing for Blockchain Consensus Implementations*

Forky is a fork state-aware differential fuzzing framework designed to detect implementation discrepancies within the critical fork-handling process. 
We test Forky on the two most influential blockchain projects: *Bitcoin* and *Ethereum*, which are the representatives of the two major blockchain consensus algorithm families, Proof-of-Work (PoW) and Proof-of-Stake (PoS) consensus algorithms.

> [!NOTE]
> The current version is a Proof-of-Concept implementation, so it is not optimized yet.

> [!WARNING]
> Current PoC implementation utilizes a large amount of disk I/O that can impact SSD lifespan.

### Forky Bitcoin
(**forky-bitcoin**) Differential testing Bitcoin fork resolution implementation with Forky
* Target client: Bitcoin Core (C++), Bitcoin Knots (C++), btcd (Go), bcoin (JavaScript)

### Forky Ethereum 1.0 (PoW) 
(**forky-eth1**) Fuzzer for testing Ethereum 1.0 (PoW) fork resolution implementation with Forky
* Target client: Geth (Go)

### Forky Ethereum 2.0 (PoS)
(**forky-eth2**) Differential testing Ethereum 2.0 fork resolution implementation with Forky
* Target client: Prysm (Go), Lighthouse (Rust), Teku (Java), Nimbus (Nim)

---
### Citation
```
@inproceedings{kim2025fork,
  title = {{Fork State-Aware Differential Fuzzing for Blockchain Consensus Implementations}},
  author = {Wonhoi Kim and Hocheol Nam and Muoi Tran and Amin Jalilov and Zhenkai Liang and Sang Kil Cha and Min Suk Kang},
  booktitle = {Proceedings of the 47th IEEE/ACM International Conference on Software Engineering (ICSE'25)},
  year = {2025},
  location = {Ottawa, Ontario, Canada},
  doi = {10.1109/ICSE55347.2025.00085},
  pages = {622-622},
  publisher = {IEEE Computer Society},
  address = {Los Alamitos, CA, USA},
  month = May
}
```
