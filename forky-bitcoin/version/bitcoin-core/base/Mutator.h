#include <validation.h>
#include <consensus/merkle.h>
#include <consensus/params.h>
#include <chainparams.h>
#include <pow.h>

#include <vector>
#include <random>
#include <chrono>
#include <algorithm>

#include "Parser.h"
#include "json.hpp"

// #define RUN_COUNT_MAX 30000
// #define RUN_COUNT_REACHED_MAX(x) (x >= RUN_COUNT_MAX)
#define INITIALIZE(x) (x == 0)
#define PRINT_UTXO true
#define MAX_BRANCH 3
#define MAX_TOTAL_BLOCK 64
#define MAX_TXN_COUNT 20

using json = nlohmann::json;

// Anonymized_Author_B: A global random generator. Do not create any other rng states
std::unique_ptr<std::mt19937_64> gRnd;

fs::path pathImport;
std::vector<std::pair<CTxIn, CAmount>> seenTxIn;
std::vector<std::pair<CTxIn, CAmount>> txInPool;
CBlockIndex *pindexFork;
std::vector<int64_t> vTestTimes;
std::vector<int64_t> vTestTimesAvg;
std::vector<int> vCaseSize;
std::vector<int64_t> vMutTimes;
std::vector<int64_t> vMutTimesAvg;
std::vector<int64_t> vRevTimes;
std::vector<int64_t> vRevTimesAvg;

std::chrono::milliseconds test_time{0}, mutate_time{0}, init_time{0}, rev_time{0};
int run_count = 0, mutation_count = 0;
int parsed = 0;
int accepted = 0, nAccept = 0;
CBlockIndex* hashTip;

bool operator<(const CTxIn& a, const CTxIn& b) {
    return (a.prevout.hash < b.prevout.hash) ||
           (a.prevout.n < b.prevout.n);
}

template <typename T>
T RandomInRange(T min, T max);
template <typename T>
T RandomInRangeByP(T soft_min, T soft_max, T hard_min, T hard_max, double pHard);
template <typename T>
T RandomInRangeByP(T soft_min, T soft_max, double pHard);
uint256 RandomUint256();

int CalculateHeight(TestCaseType& TestCase, int index);
std::vector<std::pair<CTxIn, CAmount>> CalculateVCPool(TestCaseType& TestCase, int indexTip);
std::vector<std::pair<CTxIn, CAmount>> CalculateICPool(TestCaseType& TestCase, int indexTip);

void MutateVtoV(CBlock& block, std::vector<std::pair<CTxIn, CAmount>> coinPool);
void MutateVtoI(CBlock& block, std::vector<std::pair<CTxIn, CAmount>> coinPool);
void MutateItoI(CBlock& block, std::vector<std::pair<CTxIn, CAmount>> coinPool);

CBlock generate_valid_block(CBlockIndex& pindexPrev, int height, std::vector<std::pair<CTxIn, CAmount>> coinPool);
CBlock generate_valid_block(CBlock& prevBlock, int height, std::vector<std::pair<CTxIn, CAmount>> coinPool);
CBlock generate_invalid_block(CBlock& prevBlock, int height, std::vector<std::pair<CTxIn, CAmount>> coinPool);

size_t __CustomMutator(uint8_t *Data, size_t Size, size_t MaxSize, unsigned int Seed);
                                        
void AddBlock(TestCaseType& TestCase, std::vector<int>& leaves, std::vector<int>& vleaves);
void DeleteLeaf(TestCaseType& TestCase, std::vector<int>& leaves, std::vector<int>& vleaves);
void ModifyLeaf(TestCaseType& TestCase, std::vector<int>& leaves, std::vector<int>& vleaves);

template <typename... Callables>
void CallOneOfByProb(std::vector<double>& vProb, Callables... callables);

void WriteTestCase (const unsigned char *data, size_t size);
void WriteTestCase (std::vector <uint8_t>& TestCase);

void ShuffleOrder(TestCaseType& testcase);
bool isDescendantOf(TestCaseType& testcase, int a, int b);

bool CheckTestCaseValidity(TestCaseType& testcase);

// Anonymized_Author_B: return random number in range
template <typename T>
T RandomInRange(T min, T max)
{
    static_assert (std::is_integral<T>::value, "Not an integral type.");
    static_assert(sizeof(T) <= sizeof(uint64_t), "Given type too big (> 64 bits).");

    if (min >= max) return static_cast<T>(min);

    uint64_t range = static_cast<uint64_t>(max) - min;
    uint64_t result = static_cast<uint64_t>((*gRnd)());
    
    if (min < 0 && max > 0) {
        std::vector<double> vProb{0.10, 0.10, 0.10, 0.70};
        CallOneOfByProb(vProb,
            [&] () { result = 0; },
            [&] () { result = range; },
            [&] () { result = min; },
            [&] () {
                if (range != std::numeric_limits<decltype(range)>::max())
                    result = result % (range + 1);
                else
                    result = result % range;
            }
        );
    }
    else {
        std::vector<double> vProb{0.10, 0.10, 0.80};
        CallOneOfByProb(vProb,
            [&] () { result = 0; },
            [&] () { result = range; },
            [&] () {
                if (range != std::numeric_limits<decltype(range)>::max())
                    result = result % (range + 1);
                else
                    result = result % range;
            }
        );
    }

    return static_cast<T>(min + result);
}

void WriteTestCase (const unsigned char *data, size_t size)
{
    // Anonymized_Author_A: Save mutated test case to ./test_cases/<SHA1 Hash>
    std::cout << "\t\t Write test case to test_cases..." << std::endl;
    std::vector<unsigned char> vchHash;
    vchHash.resize (CSHA1::OUTPUT_SIZE);
    CSHA1().Write (data, size).Finalize(vchHash.data());
    // fs::path pathTestCase = fs::current_path() / "test_cases" / PACKAGE_VERSION / HexStr(vchHash);
    fs::path pathTestCase = fs::current_path() / "test_cases" / HexStr(vchHash);
#if __cplusplus < 201402L
    boost::filesystem::ofstream fileTestCase(pathTestCase, std::ios::binary);
#else
    std::ofstream fileTestCase{pathTestCase, std::ios::binary};
#endif
    fileTestCase.write(reinterpret_cast<const char*>(data), int(size));
    fileTestCase.close();
}

void AddToCorpus (const unsigned char *data, size_t size)
{
    std::vector<unsigned char> vchHash;
    vchHash.resize (CSHA1::OUTPUT_SIZE);
    CSHA1().Write (data, size).Finalize(vchHash.data());
    std::cout << "\t\t Add tc to corpus, hash: " << HexStr(vchHash) << std::endl;
    // fs::path pathTestCase = fs::current_path() / "fuzzing_corpus" / HexStr(vchHash);
    fs::path pathTestCase = fs::current_path() / "reorg_corpus" / HexStr(vchHash);
#if __cplusplus < 201402L
    boost::filesystem::ofstream fileTestCase(pathTestCase, std::ios::binary);
#else
    std::ofstream fileTestCase{pathTestCase, std::ios::binary};
#endif
    fileTestCase.write(reinterpret_cast<const char*>(data), int(size));
    fileTestCase.close();
}

void AddToDir (const unsigned char *data, size_t size, fs::path path)
{
    std::vector<unsigned char> vchHash;
    vchHash.resize (CSHA1::OUTPUT_SIZE);
    CSHA1().Write (data, size).Finalize(vchHash.data());
#if __cplusplus < 201402L
    boost::filesystem::ofstream fileTestCase(path, std::ios::binary);
#else
    std::ofstream fileTestCase{path, std::ios::binary};
#endif
    fileTestCase.write(reinterpret_cast<const char*>(data), int(size));
    fileTestCase.close();
}

void WriteTestCase (std::vector <uint8_t>& TestCase)
{
    WriteTestCase (TestCase.data(), TestCase.size());
}

// Anonymized_Author_B: return random number in soft range by chance of p
template <typename T>
T RandomInRangeByP(T soft_min, T soft_max, T hard_min, T hard_max, double pHard)
{
    std::uniform_real_distribution<double> dist(0, 1);
    double p = dist((*gRnd));

    if(p < pHard) return RandomInRange(soft_min, soft_max);
    else return RandomInRange(hard_min, hard_max);
}

template <typename T>
T RandomInRangeByP(T soft_min, T soft_max, double pHard)
{
    return RandomInRangeByP(soft_min, soft_max,
                            std::numeric_limits<T>::min(),
                            std::numeric_limits<T>::max(),
                            pHard);
}

uint256 RandomUint256()
{
    uint8_t result;

    std::vector<uint8_t> vch;
    for (int i = 0; i < 32; i++) {
        result = static_cast<uint8_t>((*gRnd)());
        vch.push_back(result);
    }
    return uint256(vch);
}

int CalculateHeight(TestCaseType& TestCase, int index)
{
    int curBlock = index, height = 1;

    while (TestCase[curBlock].prev != -1)
    {
        curBlock = TestCase[curBlock].prev;
        height++;
    }

    return 200 + height;
}

// return VC pool after processing this block.
// ex) indexTip = 0 => existing blockchain + one block
// ex) indexTip = -1 => existing blockchain
std::vector<std::pair<CTxIn, CAmount>> CalculateVCPool(TestCaseType& TestCase, int indexTip)
{
    // A gloal variable representing the VC pool at block #200
    auto pool = txInPool;
    if (indexTip < 0) return pool;

    int curBlock = indexTip;
    std::vector <int> vWalk;

    while (curBlock != -1)
    {
        vWalk.push_back(curBlock);
        curBlock = TestCase[curBlock].prev;
    }

    std::reverse (vWalk.begin(), vWalk.end());

    for (auto& index : vWalk)
    {
        TestCaseBlock& block = TestCase[index];
        assert(block.isValid);

        for (auto& tx : block.block.vtx)
        {
            uint256 txid = tx->GetHash();
            int n = 0;
            for (auto& txin : tx->vin)
            {
                // std::cout << txin.prevout.ToString() << ", " << txin.nSequence << std::endl;
                auto del = std::find_if(pool.begin(), pool.end(), [&txin](const std::pair<CTxIn, CAmount>& elem) { return elem.first.prevout == txin.prevout; });
                if (del != pool.end())
                    pool.erase(del);
            }
            if (!tx->IsCoinBase()) {
                for (auto& txout : tx->vout)
                {
                    pool.push_back(std::make_pair(CTxIn{txid, static_cast<uint32_t>(n)}, txout.nValue));
                    n++;
                }
            }
        }
    }

    return pool;
}

std::vector<std::pair<CTxIn, CAmount>> CalculateICPool(TestCaseType& TestCase, int indexTip)
{
    auto pool = txInPool;
    if (indexTip < 0) return pool;
    
    int curBlock = indexTip;
    std::vector <int> vWalk;

    while (curBlock != -1)
    {
        vWalk.push_back(curBlock);
        curBlock = TestCase[curBlock].prev;
    }

    std::reverse (vWalk.begin(), vWalk.end());

    for (auto& index : vWalk)
    {
        TestCaseBlock& block = TestCase[index];
        assert(block.isValid);

        for (auto& tx : block.block.vtx)
        {
            uint256 txid = tx->GetHash();
            int n = 0;
            if (!tx->IsCoinBase()) {
                for (auto& txout : tx->vout)
                {
                    pool.push_back(std::make_pair(CTxIn{txid, static_cast<uint32_t>(n)}, txout.nValue));
                    n++;
                }
            }
        }
    }

    return pool;
}

void MutateVtoV(CBlock& block, std::vector<std::pair<CTxIn, CAmount>> coinPool)
{
    std::vector<double> vProb{0.2, 0.2, 0.8};
    if (block.vtx.size() <= 1)
        vProb = {0.5, 0.5};

    CallOneOfByProb(vProb,
        [&block] () { block.nVersion = RandomInRange(1, 4); },
        [&block] () { block.nTime = RandomInRange(block.nTime, block.nTime + 20 * 60); }, 
        [&block, &coinPool] () {
            assert (block.vtx.size() > 1);
            int indexTx = (*gRnd)() % (block.vtx.size() - 1) + 1;
            auto& refTx = block.vtx[indexTx];
            CMutableTransaction tx(*refTx);

            std::vector<double> prob{0.5, 0.5};
            CallOneOfByProb(prob,
                [&tx] () { tx.nVersion = RandomInRange(1, 2); },
                [&tx, &coinPool] () {
                    if (!coinPool.empty()) {
                        auto coin = coinPool[(*gRnd)() % coinPool.size()];
                        tx.vin.push_back(coin.first);
                    } } );

            block.vtx[indexTx] = MakeTransactionRef(tx); });

    block.nNonce = 0;
    block.hashMerkleRoot = BlockMerkleRoot(block);
    std::cout << "Start PoW\n";
    while (!CheckProofOfWork(block.GetHash(), block.nBits, Params().GetConsensus())) {
        ++block.nNonce;
    }
    std::cout << "End PoW\n";
}

void MutateVtoI(CBlock& block, std::vector<std::pair<CTxIn, CAmount>> coinPool)
{
    std::vector<double> vProb{0.05, 0.05, 0.05, 0.85};
    CallOneOfByProb(vProb,
        [&block] () { block.nVersion = RandomInRangeByP(1, 4, 0.1); }, 
        [&block] () { block.hashPrevBlock = RandomUint256(); },
        [&block] () { block.nTime = RandomInRangeByP(block.nTime, block.nTime + 20 * 60, 0.1); },
        [&block, &coinPool] () {
            if (block.vtx.empty()) return;
            int indexTx = (*gRnd)() % block.vtx.size();
            auto& refTx = block.vtx[indexTx];
            CMutableTransaction tx(*refTx);

            std::vector<double> prob{0.1, 0.1, 0.2, 0.2, 0.2, 0.2};
            CallOneOfByProb(prob,
                [&tx] () {},
                [&tx] () { tx.nVersion = RandomInRangeByP(1, 2, 0.1); },
                [&tx] () { tx.vin[(*gRnd)() % tx.vin.size()].prevout.hash = RandomUint256(); },
                [&tx] () { tx.vin[(*gRnd)() % tx.vin.size()].prevout.n = RandomInRange(std::numeric_limits<uint32_t>::min(), std::numeric_limits<uint32_t>::max()); },
                [&tx] () { tx.vout[(*gRnd)() % tx.vout.size()].nValue = RandomInRange(std::numeric_limits<int64_t>::min(), std::numeric_limits<int64_t>::max()); },
                [&tx, &coinPool] () {
                    if (!coinPool.empty()) {
                        auto coin = coinPool[(*gRnd)() % coinPool.size()];
                        tx.vin.push_back(coin.first);
                    } } );
                /* add coin mutations */
            
            block.vtx[indexTx] = MakeTransactionRef(tx); });

    block.nNonce = 0;
    block.hashMerkleRoot = BlockMerkleRoot(block);
    std::cout << "Start PoW\n";
    while (!CheckProofOfWork(block.GetHash(), block.nBits, Params().GetConsensus())) {
        ++block.nNonce;
    }
    std::cout << "End PoW\n";
}

void MutateItoI(CBlock& block, std::vector<std::pair<CTxIn, CAmount>> coinPool)
{
    std::vector<double> vProb{0.05, 0.05, 0.05, 0.85};
    CallOneOfByProb(vProb,
        [&block] () { block.nVersion = RandomInRangeByP(1, 4, 0.1); }, 
        [&block] () { block.hashPrevBlock = RandomUint256(); },
        [&block] () { block.nTime = RandomInRangeByP(block.nTime, block.nTime + 20 * 60, 0.1); },
        [&block, &coinPool] () {
            if (block.vtx.empty()) return;
            int indexTx = (*gRnd)() % block.vtx.size();
            auto& refTx = block.vtx[indexTx];
            CMutableTransaction tx(*refTx);

            std::vector<double> prob{0.1, 0.1, 0.2, 0.2, 0.2, 0.2};
            CallOneOfByProb(prob,
                [&tx] () {},
                [&tx] () { tx.nVersion = RandomInRangeByP(1, 2, 0.1); },
                [&tx] () { tx.vin[(*gRnd)() % tx.vin.size()].prevout.hash = RandomUint256(); },
                [&tx] () { tx.vin[(*gRnd)() % tx.vin.size()].prevout.n = RandomInRange(std::numeric_limits<uint32_t>::min(), std::numeric_limits<uint32_t>::max()); },
                [&tx] () { tx.vout[(*gRnd)() % tx.vout.size()].nValue = RandomInRange(std::numeric_limits<int64_t>::min(), std::numeric_limits<int64_t>::max()); },
                [&tx, &coinPool] () {
                    if (!coinPool.empty()) {
                        auto coin = coinPool[(*gRnd)() % coinPool.size()];
                        tx.vin.push_back(coin.first);
                    } } );
                /* add coin mutations */
            
            block.vtx[indexTx] = MakeTransactionRef(tx); });

    block.nNonce = 0;
    block.hashMerkleRoot = BlockMerkleRoot(block);
    std::cout << "Start PoW\n";
    while (!CheckProofOfWork(block.GetHash(), block.nBits, Params().GetConsensus())) {
        ++block.nNonce;
    }
    std::cout << "End PoW\n";
}

CBlock generate_valid_block(CBlockIndex& pindexPrev, int height, std::vector<std::pair<CTxIn, CAmount>> coinPool)
{
    {
    assert (pindexPrev.nHeight == height - 1);

    LOCK(cs_main);
    CBlock block;
    block.nVersion = RandomInRange(1, 4);
    block.hashPrevBlock = pindexPrev.GetBlockHash();
    block.nTime = RandomInRange(pindexPrev.nTime, pindexPrev.nTime + 20 * 60);
    block.nBits = pindexPrev.nBits;
    block.nNonce = 0;

    CMutableTransaction coinbaseTx;
    coinbaseTx.vin.resize(1);
    coinbaseTx.vin[0].prevout.SetNull();
    coinbaseTx.vout.resize(1);
    coinbaseTx.vout[0].scriptPubKey = CScript() << OP_TRUE;
    coinbaseTx.vout[0].nValue = GetBlockSubsidy(height, Params().GetConsensus());
    coinbaseTx.vin[0].scriptSig = CScript() << height << OP_0;
    block.vtx.push_back(MakeTransactionRef(CTransaction(coinbaseTx)));

    // Anonymized_Author_B: X~N(0, 4447.8067), P(X<3000) = 0.75
    // Therefore, the number of tx in a block is < 3000 by 50% chance
    // std::normal_distribution<double> norm(0, 4447);
    // Anonymized_Author_B: number of transactions inside the block, might change
    int txn_count = MAX_TXN_COUNT;

    while (txn_count > 0) {
        txn_count--;
        int txIn_count = (*gRnd)() % 10 + 1;
        int txOut_count = (*gRnd)() % 10 + 1;
        CMutableTransaction tx;
        CAmount nValueIn = 0, nValueOut = 0;

        if (coinPool.empty()) break;
        while (txIn_count > 0) {
            txIn_count--;
            if (coinPool.empty()) break;
            int index = (*gRnd)() % coinPool.size();
            CTxIn tx_in;

            // Select a coin from the pool
            auto it = coinPool.begin();
            std::advance(it, index);
            tx_in = it->first;
            nValueIn += it->second;
            coinPool.erase(it);
            // tx_in.nSequence = ??
            tx.vin.push_back (tx_in);
        }
        nValueOut = nValueIn;
        int _txOut_count = txOut_count;
        while (txOut_count > 0){
            txOut_count--;

            int64_t tx_out_nValue = nValueOut <= 0 ? 0 : (*gRnd)() % nValueOut;

            CTxOut tx_out(tx_out_nValue, CScript() << OP_TRUE);
            tx.vout.push_back(tx_out);
            nValueOut -= tx_out_nValue;
        }
        tx.nVersion = RandomInRange(1, 2);
        // tx.nLockTime = RandomInRange(std::numeric_limits<uint32_t>::min(), std::numeric_limits<uint32_t>::max());

        // Add newly created coins to the pool
        uint256 txid = tx.GetHash();
        for (int i = 0; i < _txOut_count; i++) {
            seenTxIn.push_back(std::make_pair(CTxIn{txid, static_cast<uint32_t>(i)}, tx.vout[i].nValue));
        }

        block.vtx.push_back(MakeTransactionRef(CTransaction(tx)));
    }

    block.hashMerkleRoot = BlockMerkleRoot(block);
    std::cout << "Start PoW\n";
    while (!CheckProofOfWork(block.GetHash(), block.nBits, Params().GetConsensus())) {
        ++block.nNonce;
    }
    std::cout << "End PoW\n";
    return block;
    }
}

CBlock generate_valid_block(CBlock& prevBlock, int height, std::vector<std::pair<CTxIn, CAmount>> coinPool)
{
    {
    LOCK(cs_main);
    CBlock block;
    block.nVersion = RandomInRange(1, 4);
    block.hashPrevBlock = prevBlock.GetHash();
    block.nTime = RandomInRange(prevBlock.nTime, prevBlock.nTime + 20 * 60);
    block.nBits = prevBlock.nBits;
    block.nNonce = 0;

    CMutableTransaction coinbaseTx;
    coinbaseTx.vin.resize(1);
    coinbaseTx.vin[0].prevout.SetNull();
    coinbaseTx.vout.resize(1);
    coinbaseTx.vout[0].scriptPubKey = CScript() << OP_TRUE;
    coinbaseTx.vout[0].nValue = GetBlockSubsidy(height, Params().GetConsensus());
    coinbaseTx.vin[0].scriptSig = CScript() << height << OP_0;
    block.vtx.push_back(MakeTransactionRef(CTransaction(coinbaseTx)));

    // Anonymized_Author_B: X~N(0, 4447.8067), P(X<3000) = 0.75
    // Therefore, the number of tx in a block is < 3000 by 50% chance
    // std::normal_distribution<double> norm(0, 4447);
    // Anonymized_Author_B: number of transactions inside the block, might change
    int txn_count = MAX_TXN_COUNT;

    while (txn_count > 0) {
        // std::cout << std::endl;
        txn_count--;
        int txIn_count = (*gRnd)() % 10 + 1;
        int txOut_count = (*gRnd)() % 10 + 1;
        CMutableTransaction tx;
        CAmount nValueIn = 0, nValueOut = 0;

        if (coinPool.empty()) break;
        while (txIn_count > 0) {
            if (coinPool.empty()) break;
            txIn_count--;
            int index = (*gRnd)() % coinPool.size();
            CTxIn tx_in;

            // Select a coin from the pool
            auto it = coinPool.begin();
            std::advance(it, index);
            tx_in = it->first;
            nValueIn += it->second;
            coinPool.erase(it);
            // tx_in.nSequence = ??
            tx.vin.push_back (tx_in);
        }
        nValueOut = nValueIn;
        int _txOut_count = txOut_count;
        while (txOut_count > 0){
            txOut_count--;

            // std::cout << "Value left: " << nValueOut;
            int64_t tx_out_nValue = nValueOut <= 0 ? 0 : (*gRnd)() % nValueOut;
            // std::cout << ", Value used: " << tx_out_nValue << std::endl;

            CTxOut tx_out(tx_out_nValue, CScript() << OP_TRUE);
            tx.vout.push_back(tx_out);
            nValueOut -= tx_out_nValue;
        }
        tx.nVersion = RandomInRange(1, 2);
        // tx.nLockTime = RandomInRange(std::numeric_limits<uint32_t>::min(), std::numeric_limits<uint32_t>::max());
        block.vtx.push_back(MakeTransactionRef(CTransaction(tx)));

        // A dd newly created coins to the pool
        uint256 txid = tx.GetHash();
        for (int i = 0; i < _txOut_count; i++) {
            seenTxIn.push_back(std::make_pair(CTxIn{txid, static_cast<uint32_t>(i)}, tx.vout[i].nValue));
        }
    }

    block.hashMerkleRoot = BlockMerkleRoot(block);
    std::cout << "Start PoW\n";
    while (!CheckProofOfWork(block.GetHash(), block.nBits, Params().GetConsensus())) {
        ++block.nNonce;
    }
    std::cout << "End PoW\n";
    return block;
    }
}

CBlock generate_invalid_block(CBlock& prevBlock, int height, std::vector<std::pair<CTxIn, CAmount>> coinPool)
{
    {
    LOCK(cs_main);
    CBlock block;

    block.nVersion = RandomInRangeByP(1, 4, 0.1);
    block.hashPrevBlock = prevBlock.GetHash();
    block.nTime = RandomInRangeByP(prevBlock.nTime, prevBlock.nTime + 20 * 60, 0.1);
    // block.nBits = RandomInRangeByP(prevBlock.nBits, prevBlock.nBits, 0.1);
    block.nBits = prevBlock.nBits;
    block.nNonce = 0;

    CMutableTransaction coinbaseTx;
    coinbaseTx.vin.resize(1);
    coinbaseTx.vin[0].prevout.SetNull();
    coinbaseTx.vout.resize(1);
    coinbaseTx.vout[0].scriptPubKey = CScript() << OP_TRUE;
    coinbaseTx.vout[0].nValue = GetBlockSubsidy(height, Params().GetConsensus());
    coinbaseTx.vin[0].scriptSig = CScript() << height << OP_0;
    block.vtx.push_back(MakeTransactionRef(CTransaction(coinbaseTx)));

    // Anonymized_Author_B: X~N(0, 4447.8067), P(X<3000) = 0.75
    // Therefore, the number of tx in a block is < 3000 by 50% chance
    // std::normal_distribution<double> norm(0, 4447);
    // Anonymized_Author_B: number of transactions inside the block, might change
    int txn_count = MAX_TXN_COUNT;

    std::vector<double> vProb{0.1, 0.9};

    while (txn_count > 0) {
        txn_count--;
        CMutableTransaction tx;

        CallOneOfByProb(vProb,
            [&] () {
                auto& pTx = prevBlock.vtx[(*gRnd)() % prevBlock.vtx.size()];
                tx.vin = pTx->vin;
                tx.vout = pTx->vout;
                tx.nVersion = pTx->nVersion;
                tx.nLockTime = pTx->nLockTime;
            },
            [&] () {
                int txIn_count = (*gRnd)() % 10 + 1;
                int txOut_count = (*gRnd)() % 10 + 1;
        auto coinStart = std::chrono::high_resolution_clock::now();
                while (txIn_count > 0) {
                    txIn_count--;
                    CTxIn tx_in;
                    tx_in.prevout.hash = RandomUint256();
                    tx_in.prevout.n = RandomInRange(std::numeric_limits<uint32_t>::min(), std::numeric_limits<uint32_t>::max());
                    // tx_in.nSequence;
                    tx.vin.push_back(tx_in);
                }
                while (txOut_count > 0){
                    txOut_count--;

                    int64_t tx_out_nValue = RandomInRange(std::numeric_limits<int64_t>::min(), std::numeric_limits<int64_t>::max());
                    CTxOut tx_out(tx_out_nValue, CScript() << OP_TRUE);

                    tx.vout.push_back(tx_out);
                }
                tx.nVersion = RandomInRangeByP(1, 2, 0.1);
                // tx.nLockTime = RandomInRange(std::numeric_limits<uint32_t>::min(), std::numeric_limits<uint32_t>::max());
            });
            
        block.vtx.push_back(MakeTransactionRef(CTransaction(tx)));
    }

    block.nNonce = 0;
    block.hashMerkleRoot = BlockMerkleRoot(block);
    std::cout << "Start PoW\n";
    while (!CheckProofOfWork(block.GetHash(), block.nBits, Params().GetConsensus())) {
        ++block.nNonce;
    }
    std::cout << "End PoW\n";
    return block;
    }
}

void RandomAddBlock(TestCaseType& TestCase, std::vector<int>& leaves, std::vector<int>& vleaves)
{
    int nBlocks = TestCase.size();
    std::cout << "\tAdd a block\n";
    if (nBlocks >= MAX_TOTAL_BLOCK) return;
    int indexBlock;

    do {
        indexBlock = (*gRnd)() % TestCase.size();
    } while (!TestCase[indexBlock].isValid);
    // indexBlock = leaves[0];

    assert(TestCase[indexBlock].isValid);
    
    std::vector<std::pair<CTxIn, CAmount>> coinPool = CalculateVCPool(TestCase, indexBlock);
    CBlock newBlock;
    bool generateValidBlock;

    std::vector<double> vProb{0.00, 1.00};
    CallOneOfByProb(vProb,
        [&generateValidBlock, &newBlock, &TestCase, &indexBlock, &coinPool] () { generateValidBlock = true; newBlock = generate_valid_block(TestCase[indexBlock].block, CalculateHeight(TestCase, indexBlock) + 1, coinPool); },
        [&generateValidBlock, &newBlock, &TestCase, &indexBlock, &coinPool] () { generateValidBlock = false; newBlock = generate_invalid_block(TestCase[indexBlock].block, CalculateHeight(TestCase, indexBlock) + 1, coinPool); });

    TestCase[indexBlock].nChild += 1;    auto pos = TestCase.begin();
    TestCase.insert((pos + indexBlock + 1), TestCaseBlock(newBlock, generateValidBlock, 0, indexBlock));

    for (int i = indexBlock + 2; i < TestCase.size(); i++)
    {
        if (TestCase[i].prev > indexBlock) TestCase[i].prev++;
    }
    // TestCase.push_back(TestCaseBlock(newBlock, generateValidBlock, 0, indexBlock));
}

void AddBlock(TestCaseType& TestCase, std::vector<int>& leaves, std::vector<int>& vleaves)
{
    int nBlocks = TestCase.size();
    std::cout << "\tAdd a block\n";
    if (nBlocks >= MAX_TOTAL_BLOCK) return;
    int indexBlock;

    auto _vleaves = vleaves;
    if (vleaves.size() >= 2) {
        std::vector<double> vProb2{0.7, 0.3};
        CallOneOfByProb(vProb2,
            [&indexBlock, &leaves, &TestCase] () { 
            std::cout << "Do not promote long-lived competition\n";
                do {
                    indexBlock = (*gRnd)() % TestCase.size();
                } while (!TestCase[indexBlock].isValid); },
            [&indexBlock, &_vleaves, &TestCase] () { 
                // Promote long-lived competition
                std::cout << "Promote long-lived competition\n";
                std::sort(_vleaves.begin(), _vleaves.end(), [&TestCase] (int a, int b) { return CalculateHeight(TestCase, a) > CalculateHeight(TestCase, b); });
                indexBlock = _vleaves[1];
                });
    }
    else {
        do {
            indexBlock = (*gRnd)() % TestCase.size();
        } while (!TestCase[indexBlock].isValid);
    }


    assert(TestCase[indexBlock].isValid);
    
    std::vector<std::pair<CTxIn, CAmount>> coinPool = CalculateVCPool(TestCase, indexBlock);
    CBlock newBlock;
    bool generateValidBlock;
    std::cout << "Generate new block\n";
    std::vector<double> vProb{0.8, 0.2};
    CallOneOfByProb(vProb,
        [&generateValidBlock, &newBlock, &TestCase, &indexBlock, &coinPool] () { generateValidBlock = true; newBlock = generate_valid_block(TestCase[indexBlock].block, CalculateHeight(TestCase, indexBlock) + 1, coinPool); },
        [&generateValidBlock, &newBlock, &TestCase, &indexBlock, &coinPool] () { generateValidBlock = false; newBlock = generate_invalid_block(TestCase[indexBlock].block, CalculateHeight(TestCase, indexBlock) + 1, coinPool); });

    TestCase[indexBlock].nChild += 1;    auto pos = TestCase.begin();
    TestCase.insert((pos + indexBlock + 1), TestCaseBlock(newBlock, generateValidBlock, 0, indexBlock));

    for (int i = indexBlock + 2; i < TestCase.size(); i++)
    {
        if (TestCase[i].prev > indexBlock) TestCase[i].prev++;
    }
    // TestCase.push_back(TestCaseBlock(newBlock, generateValidBlock, 0, indexBlock));
}

void DeleteLeaf(TestCaseType& TestCase, std::vector<int>& leaves, std::vector<int>& vleaves)
{
    std::cout << "\tDelete a leaf block\n";
    int nBlocks = TestCase.size();
    if (nBlocks <= 1) return;
    int indexBlock = leaves[(*gRnd)() % leaves.size()];
    int indexParent = TestCase[indexBlock].prev;
    std::cout << "Delete index: " << indexBlock << std::endl;
    assert(TestCase[indexBlock].isLeaf());

    // Anonymized_Author_B: this is a bug, parent shouldn't be unconditionally set as leaf
    // TestCase[indexParent].isLeaf = true;

    TestCase[indexParent].nChild -= 1;

    for (int i = indexBlock + 1; i < nBlocks; i++)
        if (TestCase[i].prev >= indexBlock)
            TestCase[i].prev --;

    TestCase.erase(TestCase.begin() + indexBlock);
}

void ModifyLeaf(TestCaseType& TestCase, std::vector<int>& leaves, std::vector<int>& vleaves)
{
    std::cout << "\tModify a leaf block\n";
    int nBlocks = TestCase.size();
    if (nBlocks < 1) return;
    int indexBlock = leaves[(*gRnd)() % leaves.size()];
    assert(TestCase[indexBlock].isLeaf());

    bool& fValid = TestCase[indexBlock].isValid;
    int parent = TestCase[indexBlock].prev;

    std::vector<double> vProb{0.50, 0.50};
    CallOneOfByProb(vProb,
        [&] () { 
            if (fValid) {
                std::cout << "\tV to V\n";
                auto vcpool = CalculateVCPool(TestCase, parent);

                for (auto& tx : TestCase[indexBlock].block.vtx)
                {
                    for (auto& txin : tx->vin)
                    {
                        auto del = std::find_if(vcpool.begin(), vcpool.end(), [&txin] (const std::pair<CTxIn, CAmount>& elem) { if (elem.first.prevout == txin.prevout) return true; else return false; });
                        if (del != vcpool.end())
                            vcpool.erase(del);
                    }
                }
                MutateVtoV(TestCase[indexBlock].block, vcpool);
            }
            else {
                std::cout << "\tI to V\n";
                // for (int i = indexBlock + 1; i < nBlocks; i++)
                //     if (TestCase[i].prev >= indexBlock)
                //         TestCase[i].prev --;

                CBlock newBlock;
                newBlock = generate_valid_block(TestCase[parent].block,
                                                CalculateHeight(TestCase, indexBlock),
                                                CalculateVCPool(TestCase, parent));
                TestCase[indexBlock].block = newBlock;
                TestCase[indexBlock].isValid = true;
                TestCase[indexBlock].nChild = 0;
                TestCase[indexBlock].prev = parent;
                // TestCase.erase(TestCase.begin() + indexBlock);
                // TestCase.push_back(TestCaseBlock(newBlock, true, 0, parent));
            }
        },
        [&] () {
            if (fValid) {
                std::cout << "\tV to I\n";
                MutateVtoI(TestCase[indexBlock].block, CalculateICPool(TestCase, parent));
                TestCase[indexBlock].isValid = false;
            }
            else {
                std::cout << "\tI to I\n";
                MutateItoI(TestCase[indexBlock].block, CalculateICPool(TestCase, parent));
                TestCase[indexBlock].isValid = false;
            }
        });
}

bool isDescendantOf(TestCaseType& TestCase, int target, int parent)
{
    while (target != -1)
    {
        if (target == parent) return true;
        target = TestCase[target].prev;
    }
    return false;
}

void ShuffleOrder(TestCaseType& TestCase)
{
    int nBlocks = TestCase.size();
    std::vector<int> common_parents;
    for (int i = 0; i < nBlocks; i++)
        if (TestCase[i].nChild > 1) common_parents.push_back(i);

    if (common_parents.empty()) return;

    std::cout << "\tShuffle submission order\n";
    int parent = common_parents[(*gRnd)() % common_parents.size()];
    std::cout << "BEFORE\n";
    PrintTestCase(TestCase);

    std::cout << "\tParent index: " << parent << std::endl;

    std::vector <int> childs;
    for (int i = 0; i < nBlocks; i++)
        if (TestCase[i].prev == parent) childs.push_back(i);
    std::cout << "\tChilds " << childs.size() << " : ";
    for (auto i : childs)
        std::cout << i << ", ";
    std::cout << "\n";

    assert (childs.size() > 1);
    int nf = (*gRnd)() % childs.size();
    int nr = (*gRnd)() % (childs.size() - 1);
    if (nr >= nf) nr++;
    assert (nf != nr);
    if (nf > nr) std::swap(nf, nr);

    int nFront = childs[nf];
    int nRear = childs[nr];
    int nFrontEnd = (nr == nf + 1) ? nRear : childs[nf + 1];
    int nRearEnd = (nr == (childs.size() - 1)) ? -1 : childs[nr + 1];
    if (nRearEnd == -1)
    {
        for (int i = nRear; i < nBlocks; i++)
        {
            if (!isDescendantOf(TestCase, i, nRear))
            {
                nRearEnd = i;
                break;
            }
        }
        if (nRearEnd == -1) nRearEnd = nBlocks;
    }

    TestCaseType newTestCase;
    for (int i = 0; i < nFront; i++)
        newTestCase.push_back(TestCase[i]);

    // 1. rear subtree, - (nRear - nFront)
    for (int i = nRear; i < nRearEnd; i++)
    {
        if (!(i == nRear)) 
            TestCase[i].prev = TestCase[i].prev - (nRear - nFront);
        newTestCase.push_back(TestCase[i]);
    }

    // 2. else subtrees, - len(front) + len(rear)
    //                 = - (nFrontEnd - nFront) + (nRearEnd - nRear)
    for (int i = nFrontEnd; i < nRear; i++)
    {
        if (!(std::find(childs.begin(), childs.end(), i) != childs.end()))
            TestCase[i].prev = TestCase[i].prev - (nFrontEnd - nFront) + (nRearEnd - nRear);
        newTestCase.push_back(TestCase[i]);
    }

    // 3. front subtree, + len(rear) + len(else)
    //                 = + (nRearEnd - nRear) + (nRear - nFrontEnd)
    for (int i = nFront; i < nFrontEnd; i++)
    {
        if (!(i == nFront))
            TestCase[i].prev = TestCase[i].prev + (nRearEnd - nRear) + (nRear - nFrontEnd);
        newTestCase.push_back(TestCase[i]);
    }

    for (int i = nRearEnd; i < nBlocks; i++)
        newTestCase.push_back(TestCase[i]);

    TestCase = newTestCase;
    std::cout << "AFTER\n";
    PrintTestCase(TestCase);
}

void ShuffleOrderNew(TestCaseType& TestCase)
{
    std::vector<std::pair<int, int>> pairs;
    int n = TestCase.size();
    for (int i = 0; i < n; i++) {
        for (int j = i + 1; j < n; j++) {
            pairs.push_back(std::make_pair(i, j));
        }
    }
    std::shuffle (pairs.begin(), pairs.end(), *gRnd);

    for (auto& pair : pairs)
    {
        std::cout << "Try to swap " << pair.first << " and " << pair.second << "\n";
        // 1. Choose 2 random blocks
        int iFirst = pair.first;
        int iSecond = pair.second;

        // 2. If two blocks are each other's descendant/ancessor, return
        if (isDescendantOf(TestCase, iFirst, iSecond) || isDescendantOf(TestCase, iSecond, iFirst)) continue;

        // 3. If a block will come earlier than its parent, return
        if (iFirst <= TestCase[iSecond].prev) continue;
        if (iSecond <= TestCase[iFirst].prev) continue;

        // 4. If a block will come later than its children, return
        // 4-1. Find the block's earliest child
        int iFirstChild = -1, iSecondChild = -1;
        for (int i = 0; i < n; i++)
        {
            if (TestCase[i].prev == iFirst) { iFirstChild = i; break; }
        }
        for (int i = 0; i < n; i++)
        {
            if (TestCase[i].prev == iSecond) { iSecondChild = i; break; }
        }
        // 4-2. If a block will come later than its children, return
        if (iFirstChild != -1 && iFirstChild < iSecond) continue;
        if (iSecondChild != -1 && iSecondChild < iFirst) continue;

        // 5. Swap the entries of the vector
        std::swap(TestCase[iFirst], TestCase[iSecond]);

        // 6. Update children's prev
        for (int i = 0; i < TestCase.size(); i++)
        {
            if (TestCase[i].prev == iFirst) TestCase[i].prev = iSecond;
            else if (TestCase[i].prev == iSecond) TestCase[i].prev = iFirst;
        }
    }
}

template <typename... Callables>
void CallOneOfByProb(std::vector<double>& vProb, Callables... callables)
{
    std::uniform_real_distribution<double> dist(0, 1);
    double p = dist(*gRnd);
    
    double cum = 0.0;
    int i = 0;
    bool f = false;
    
#if __cplusplus < 201402L
    __attribute__ ((unused)) int dummy[] = {0,
      (([&] (int t) {cum += vProb[i++]; if (p <= cum && !f) { callables(); f = true; return t;} return t;})(0), 0) ...};
#else
    ([&] (auto& callable) {
        cum += vProb[i++];
        if (p <= cum && !f) {callable(); f = true;}
    } (callables), ...);
#endif
}

size_t __RandomMutator(uint8_t *Data, size_t Size, size_t MaxSize, unsigned int Seed)
{
    // Anonymized_Author_B: Generate global random generator
    if (mutation_count < 1)
    {
        std::random_device rd;
#if __cplusplus < 201402L
        gRnd = std::unique_ptr<std::mt19937_64>(new std::mt19937_64(rd()));
#else
        gRnd = std::make_unique<std::mt19937_64>(rd());
#endif
    }
    
    mutation_count++;
    auto start = std::chrono::high_resolution_clock::now();
    auto tStart = std::chrono::high_resolution_clock::now();;
    auto tEnd = tStart;

    // Given the same seed the same mutation will happen
    // std::mt19937 rnd(Seed);
    gRnd->seed(Seed);

    std::cout << "\n--- Execute mutator BEFORE " << run_count << "---\n";
    tStart = std::chrono::high_resolution_clock::now();
    TestCaseType TestCase = ParseInput (Data, Size);
    tEnd = std::chrono::high_resolution_clock::now();
    std::cout << "\tParse time: " << (std::chrono::duration_cast<std::chrono::milliseconds>(tEnd - tStart)).count() << "ms" << std::endl;
    std::cout << "\tSize: " << Size << ", MaxSize: " << MaxSize << "\n";
    PrintTestCase (TestCase);

    std::vector<std::pair<CTxIn, CAmount>> existingPool = txInPool;
    if (TestCase.empty())
    {
        std::cout << "\tTest case is empty, returning 1-block test case\n";

        CBlock newBlock = generate_valid_block(*hashTip, 201, existingPool);
        TestCase.push_back(TestCaseBlock(newBlock, true, 0, -1));

        auto Mutated = SerializeTestCase (TestCase);
        std::cout << "\t1-block of size: " << Mutated.size() << ", Data address at: " << (void*)Data << "\n";
        memcpy (Data, Mutated.data(), Mutated.size());
        return Mutated.size();
    }

    assert (!TestCase.empty());
    int nBlocks = TestCase.size();

    TestCaseType victim = TestCase;
    std::vector<int> leaves, vleaves;
    for (int i = 0; i < nBlocks; i++) {
        if (TestCase[i].isLeaf()) leaves.push_back(i);
        if (TestCase[i].isLeaf() && TestCase[i].isValid) vleaves.push_back(i);
    }

    if (vleaves.empty())
    {
        std::cout << "\tNo valid leaf\n";
        int indexBlock = leaves[(*gRnd)() % leaves.size()];

        // for (int i = indexBlock + 1; i < nBlocks; i++)
        //     if (TestCase[i].prev >= indexBlock)
        //         TestCase[i].prev --;

        CBlock newBlock;
        int parent = TestCase[indexBlock].prev;
        std::vector<std::pair<CTxIn, CAmount>> pool = CalculateVCPool(TestCase, parent);
        if (parent < 0) {
            newBlock = generate_valid_block(*hashTip, 201, pool);
        }
        else {
            newBlock = generate_valid_block(TestCase[parent].block,
                                            CalculateHeight(TestCase, indexBlock),
                                            pool);
        }
        TestCase[indexBlock].block = newBlock;
        TestCase[indexBlock].isValid = true;
        TestCase[indexBlock].nChild = 0;
        TestCase[indexBlock].prev = parent;
        // TestCase.erase(TestCase.begin() + indexBlock);
        // TestCase.push_back(TestCaseBlock(newBlock, true, 0, parent));
    }

    // ModifyLeaf(TestCase, leaves, vleaves);
    else {
        std::vector<double> vProb{0.5, 0.5};
        do {
            CallOneOfByProb(vProb,
                [&] () { RandomAddBlock(TestCase, leaves, vleaves); },
                [&] () { DeleteLeaf(TestCase, leaves, vleaves); });
        } while (GetNumberOfBlocks(TestCase) > MAX_TOTAL_BLOCK);
    }

    auto end = std::chrono::high_resolution_clock::now();
    mutate_time += std::chrono::duration_cast<std::chrono::milliseconds>(end - start);
    std::cout << "\tMutation time: " << (std::chrono::duration_cast<std::chrono::milliseconds>(end - start)).count() << "ms" << std::endl;
    std::cout << "\tMutation time avg: " << (std::chrono::duration_cast<std::chrono::milliseconds>(mutate_time)).count() / mutation_count << "ms" << std::endl;
    vMutTimes.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(end - start)).count());
    vMutTimesAvg.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(mutate_time)).count() / run_count);

    auto Mutated = SerializeTestCase (TestCase);
    std::cout << "Mutated size: " << Mutated.size();
    memcpy (Data, Mutated.data(), Mutated.size());
    
    WriteTestCase(Mutated);

    std::cout << "\tMutator return\n";
    // PrintTestCase(TestCase);
    return Mutated.size();
}

size_t  __CustomMutator(uint8_t *Data, size_t Size, size_t MaxSize, unsigned int Seed)
{
    // Anonymized_Author_B: Generate global random generator
    if (mutation_count < 1)
    {
        std::random_device rd;
#if __cplusplus < 201402L
        gRnd = std::unique_ptr<std::mt19937_64>(new std::mt19937_64(rd()));
#else
        gRnd = std::make_unique<std::mt19937_64>(rd());
#endif
    }
    
    mutation_count++;
    auto start = std::chrono::high_resolution_clock::now();
    auto tStart = std::chrono::high_resolution_clock::now();;
    auto tEnd = tStart;

    // Given the same seed the same mutation will happen
    // std::mt19937 rnd(Seed);
    gRnd->seed(Seed);

    std::cout << "\n--- Execute mutator BEFORE " << run_count << "---\n";
    tStart = std::chrono::high_resolution_clock::now();
    TestCaseType TestCase = ParseInput (Data, Size);
    tEnd = std::chrono::high_resolution_clock::now();
    std::cout << "\tParse time: " << (std::chrono::duration_cast<std::chrono::milliseconds>(tEnd - tStart)).count() << "ms" << std::endl;
    std::cout << "\tSize: " << Size << ", MaxSize: " << MaxSize << "\n";
    PrintTestCase (TestCase);

    std::vector<std::pair<CTxIn, CAmount>> existingPool = txInPool;
    if (TestCase.empty())
    {
        std::cout << "\tTest case is empty, returning 3-block test case\n";

        CBlock newBlock1 = generate_valid_block(*hashTip, 201, existingPool);
        TestCase.push_back(TestCaseBlock(newBlock1, true, 2, -1));
        CBlock newBlock2 = generate_valid_block(newBlock1, 202, CalculateVCPool(TestCase, 0));
        CBlock newBlock3 = generate_valid_block(newBlock1, 202, CalculateVCPool(TestCase, 0));
        TestCase.push_back(TestCaseBlock(newBlock2, true, 0, 0));
        TestCase.push_back(TestCaseBlock(newBlock3, true, 0, 0));

        auto Mutated = SerializeTestCase (TestCase);
        std::cout << "\t3-block of size: " << Mutated.size() << ", Data address at: " << (void*)Data << "\n";
        memcpy (Data, Mutated.data(), Mutated.size());
        return Mutated.size();
    }

    assert (!TestCase.empty());
    int nBlocks = TestCase.size();

    TestCaseType victim = TestCase;
    std::vector<int> leaves, vleaves;
    for (int i = 0; i < nBlocks; i++) {
        if (TestCase[i].isLeaf()) leaves.push_back(i);
        if (TestCase[i].isLeaf() && TestCase[i].isValid) vleaves.push_back(i);
    }

    if (vleaves.empty())
    {
        std::cout << "\tNo valid leaf\n";
        int indexBlock = leaves[(*gRnd)() % leaves.size()];

        // for (int i = indexBlock + 1; i < nBlocks; i++)
        //     if (TestCase[i].prev >= indexBlock)
        //         TestCase[i].prev --;

        CBlock newBlock;
        int parent = TestCase[indexBlock].prev;
        std::vector<std::pair<CTxIn, CAmount>> pool = CalculateVCPool(TestCase, parent);
        if (parent < 0) {
            newBlock = generate_valid_block(*hashTip, 201, pool);
        }
        else {
            newBlock = generate_valid_block(TestCase[parent].block,
                                            CalculateHeight(TestCase, indexBlock),
                                            pool);
        }
        TestCase[indexBlock].block = newBlock;
        TestCase[indexBlock].isValid = true;
        TestCase[indexBlock].nChild = 0;
        TestCase[indexBlock].prev = parent;
        // TestCase.erase(TestCase.begin() + indexBlock);
        // TestCase.push_back(TestCaseBlock(newBlock, true, 0, parent));

        ///////////////////////////////////
        auto end = std::chrono::high_resolution_clock::now();
        mutate_time += std::chrono::duration_cast<std::chrono::milliseconds>(end - start);
        std::cout << "\tMutation time: " << (std::chrono::duration_cast<std::chrono::milliseconds>(end - start)).count() << "ms" << std::endl;
        std::cout << "\tMutation time avg: " << (std::chrono::duration_cast<std::chrono::milliseconds>(mutate_time)).count() / mutation_count << "ms" << std::endl;
        vMutTimes.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(end - start)).count());
        vMutTimesAvg.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(mutate_time)).count() / run_count);

        auto Mutated = SerializeTestCase (TestCase);
        std::cout << "Mutated size: " << Mutated.size();
        memcpy (Data, Mutated.data(), Mutated.size());
        
        WriteTestCase(Mutated);

        std::cout << "\tMutator return\n";
        // PrintTestCase(TestCase);
        return Mutated.size();
        ///////////////////////////////////
    }

    std::vector<double> vProb{0.8, 0.2};
    do {
        CallOneOfByProb(vProb,
            [&] () { AddBlock(TestCase, leaves, vleaves); },
            [&] () { DeleteLeaf(TestCase, leaves, vleaves); });
    } while (GetNumberOfBlocks(TestCase) > MAX_TOTAL_BLOCK);

    // Block Tree Topology mutation

    nBlocks = TestCase.size();
    vleaves.clear();
    leaves.clear();
    for (int i = 0; i < nBlocks; i++) {
        if (TestCase[i].isLeaf()) leaves.push_back(i);
        if (TestCase[i].isLeaf() && TestCase[i].isValid) vleaves.push_back(i);
    }

    if (vleaves.empty())
    {
        std::cout << "\tNo valid leaf\n";
        int indexBlock = leaves[(*gRnd)() % leaves.size()];

        // for (int i = indexBlock + 1; i < nBlocks; i++)
        //     if (TestCase[i].prev >= indexBlock)
        //         TestCase[i].prev --;

        CBlock newBlock;
        int parent = TestCase[indexBlock].prev;
        std::vector<std::pair<CTxIn, CAmount>> pool = CalculateVCPool(TestCase, parent);
        if (parent < 0) {
            newBlock = generate_valid_block(*hashTip, 201, pool);
        }
        else {
            newBlock = generate_valid_block(TestCase[parent].block,
                                            CalculateHeight(TestCase, indexBlock),
                                            pool);
        }
        TestCase[indexBlock].block = newBlock;
        TestCase[indexBlock].isValid = true;
        TestCase[indexBlock].nChild = 0;
        TestCase[indexBlock].prev = parent;
        // TestCase.erase(TestCase.begin() + indexBlock);
        // TestCase.push_back(TestCaseBlock(newBlock, true, 0, parent));
    }

    // ModifyLeaf(TestCase, leaves, vleaves);
    else {
        std::cout << "Block timing mutation\n";
        PrintTestCase(TestCase);
        std::vector<double> vProb{0.2, 0.8};
        do {
            CallOneOfByProb(vProb,
                [&] () { ModifyLeaf(TestCase, leaves, vleaves); },
                [&] () { ShuffleOrderNew(TestCase); });
        } while (GetNumberOfBlocks(TestCase) > MAX_TOTAL_BLOCK);
    }

    if (!CheckTestCaseValidity(TestCase)) {
        PrintTestCase(TestCase);
        std::cout << "Test case is invalid\n";
        assert(false);
    }

    auto end = std::chrono::high_resolution_clock::now();
    mutate_time += std::chrono::duration_cast<std::chrono::milliseconds>(end - start);
    std::cout << "\tMutation time: " << (std::chrono::duration_cast<std::chrono::milliseconds>(end - start)).count() << "ms" << std::endl;
    std::cout << "\tMutation time avg: " << (std::chrono::duration_cast<std::chrono::milliseconds>(mutate_time)).count() / mutation_count << "ms" << std::endl;
    vMutTimes.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(end - start)).count());
    vMutTimesAvg.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(mutate_time)).count() / run_count);

    auto Mutated = SerializeTestCase (TestCase);
    std::cout << "Mutated size: " << Mutated.size();
    memcpy (Data, Mutated.data(), Mutated.size());
    
    WriteTestCase(Mutated);

    std::cout << "\tMutator return\n";
    // PrintTestCase(TestCase);
    return Mutated.size();
}

bool CheckTestCaseValidity(TestCaseType& TestCase)
{
    int nBlocks = TestCase.size();

    for (int i = 0 ; i < nBlocks; i++)
    {
        // Return false if prev is bigger than i
        if (TestCase[i].prev >= i) return false;
        if (TestCase[i].isValid == false && TestCase[i].nChild > 0) return false;
    }
    return true;
}