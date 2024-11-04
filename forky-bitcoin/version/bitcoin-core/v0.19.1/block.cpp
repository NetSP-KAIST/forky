// v0.19.0
// Copyright (c) 2019-2020 The Bitcoin Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

#include <arith_uint256.h>
#include <miner.h>
#include <pow.h>
#include <chainparams.h>
#include <consensus/merkle.h>
#include <consensus/validation.h>
#include <primitives/transaction.h>

#include <core_io.h>
#include <core_memusage.h>
#include <primitives/block.h>
#include <pubkey.h>
#include <streams.h>
//#include <test/fuzz/fuzz.h>
#include <test/setup_common.h>
#include <test/util.h>
#include <validation.h>
#include <version.h>
#include <fs.h>
#include <flatfile.h>
#include <interfaces/chain.h>

#include <crypto/sha1.h>
#include <util/strencodings.h>

#include <test/fuzz/FuzzedDataProvider.h>
#include <cassert>
#include <string>
#include <random>
#include <chrono>

#include "Parser.h"
#include "json.hpp"
#include "Mutator.h"

namespace {
const TestingSetup* g_setup;
} // namespace

//! Helper for findBlock to selectively return pieces of block data.
class FoundBlock
{
public:
    FoundBlock& hash(uint256& hash) { m_hash = &hash; return *this; }
    FoundBlock& height(int& height) { m_height = &height; return *this; }
    FoundBlock& time(int64_t& time) { m_time = &time; return *this; }
    FoundBlock& maxTime(int64_t& max_time) { m_max_time = &max_time; return *this; }
    FoundBlock& mtpTime(int64_t& mtp_time) { m_mtp_time = &mtp_time; return *this; }
    //! Read block data from disk. If the block exists but doesn't have data
    //! (for example due to pruning), the CBlock variable will be set to null.
    FoundBlock& data(CBlock& data) { m_data = &data; return *this; }

    uint256* m_hash = nullptr;
    int* m_height = nullptr;
    int64_t* m_time = nullptr;
    int64_t* m_max_time = nullptr;
    int64_t* m_mtp_time = nullptr;
    CBlock* m_data = nullptr;
};

std::vector<CBlockIndex *> rev_m_chain;
int32_t rev_nBlockSequenceId;
int32_t rev_nBlockReverseSequenceId;
arith_uint256 rev_nLastPreciousChainwork;
std::atomic<bool> rev_m_cached_finished_ibd{false};
size_t rev_m_coinsdb_cache_size_bytes{0};
size_t rev_m_coinstip_cache_size_bytes{0};
std::set<CBlockIndex *, CBlockIndexWorkComparator> rev_setBlockIndexCandidates;

std::set<CBlockIndex *> rev_m_failed_blocks;
std::multimap<CBlockIndex *, CBlockIndex*> rev_m_blocks_unlinked;

std::shared_ptr<CCoinsViewCache> rev_m_cacheview;

    typedef boost::multi_index_container<
        CTxMemPoolEntry,
        boost::multi_index::indexed_by<
            // sorted by txid
            boost::multi_index::hashed_unique<mempoolentry_txid, SaltedTxidHasher>,
            // sorted by fee rate
            boost::multi_index::ordered_non_unique<
                boost::multi_index::tag<descendant_score>,
                boost::multi_index::identity<CTxMemPoolEntry>,
                CompareTxMemPoolEntryByDescendantScore
            >,
            // sorted by entry time
            boost::multi_index::ordered_non_unique<
                boost::multi_index::tag<entry_time>,
                boost::multi_index::identity<CTxMemPoolEntry>,
                CompareTxMemPoolEntryByEntryTime
            >,
            // sorted by fee rate with ancestors
            boost::multi_index::ordered_non_unique<
                boost::multi_index::tag<ancestor_score>,
                boost::multi_index::identity<CTxMemPoolEntry>,
                CompareTxMemPoolEntryByAncestorFee
            >
        >
    > indexed_transaction_set;

std::atomic<unsigned int> rev_nTransactionsUpdated{0};
uint64_t rev_totalTxSize;
CAmount rev_m_total_fee;
uint64_t rev_cachedInnerUsage;
uint64_t rev_lastRollingFreeUpdate;
bool rev_blockSinceLastRollingFeeBump;
double rev_rollingMinimumFeeRate;
// Epoch rev_m_epoch;
uint64_t rev_m_sequence_number{1};
bool rev_m_is_loaded{false};
indexed_transaction_set rev_mapTx;
using txiter = indexed_transaction_set::nth_index<0>::type::const_iterator;
std::vector<std::pair<uint256, txiter>> rev_vTxHashes;
typedef std::set<txiter, CTxMemPool::CompareIteratorByHash> setEntries;
typedef std::map<txiter, setEntries, CTxMemPool::CompareIteratorByHash> cacheMap;
std::set<uint256> rev_m_unbroadcast_txids;
indirectmap<COutPoint, const CTransaction*> rev_mapNextTx;
std::map<uint256, CAmount> rev_mapDeltas;

extern CBlockIndex* pindexBestInvalid;
extern CTxMemPool mempool;

extern std::vector<CBlockFileInfo> vinfoBlockFile;
extern int nLastBlockFile;
extern std::set<CBlockIndex*> setDirtyBlockIndex;
extern std::set<int> setDirtyFileInfo;

std::vector<CBlockFileInfo> rev_vinfoBlockFile;
int rev_nLastBlockFile;
std::set<CBlockIndex *> rev_setDirtyBlockIndex;
std::set<int> rev_setDirtyFileInfo;

CBlockIndex* rev_pindexBestHeader = nullptr;
uint256 rev_g_best_block;
int64_t rev_nMaxTipAge = DEFAULT_MAX_TIP_AGE;
uint256 rev_hashAssumeValid;
arith_uint256 rev_nMinimumChainWork;
CFeeRate rev_minRelayTxFee = CFeeRate(DEFAULT_MIN_RELAY_TX_FEE);
CBlockIndex* rev_pindexBestInvalid = nullptr;

std::unordered_map<uint256, CBlockIndex, BlockHasher> rev_BlockIndexes;

CCoinsView rev_backed_base;

void SaveBlockChain()
{
    auto& blockman = ChainstateActive().m_blockman;

    // step 0 : save ALL validation.cpp global parameters
    rev_pindexBestHeader = pindexBestHeader;
    rev_g_best_block = g_best_block;
    rev_nMaxTipAge = nMaxTipAge;
    rev_hashAssumeValid = hashAssumeValid;
    rev_nMinimumChainWork = nMinimumChainWork;
    rev_minRelayTxFee = minRelayTxFee;
    rev_pindexBestInvalid = pindexBestInvalid;

    LOCK(cs_main);
    // TODO: Do I need deep copy of CBlockIndex instances + disk file locations?
    // step 1 : save ALL block indexes.
    // Later, any block index NOT in rev_BlockIndexes MUST be erased
    for (auto& index : blockman.m_block_index) {
        rev_BlockIndexes.insert(std::make_pair(index.first, CBlockIndex(*index.second)));
    }

    // step 2 : save chainstate
    auto& cs = ChainstateActive();
    rev_m_chain = cs.m_chain.vChain;
    rev_nBlockSequenceId = cs.nBlockSequenceId;
    rev_nBlockReverseSequenceId = cs.nBlockReverseSequenceId;
    rev_nLastPreciousChainwork = cs.nLastPreciousChainwork;
    rev_m_cached_finished_ibd = cs.m_cached_finished_ibd.load();
    // rev_m_coinsdb_cache_size_bytes = cs.m_coinsdb_cache_size_bytes;
    // rev_m_coinstip_cache_size_bytes = cs.m_coinstip_cache_size_bytes;
    rev_setBlockIndexCandidates = cs.setBlockIndexCandidates;

    // step 3 : save block manager
    auto& bm = cs.m_blockman;
    rev_m_failed_blocks = bm.m_failed_blocks;
    rev_m_blocks_unlinked = bm.m_blocks_unlinked;

    // step 4 : save UTXO cache
    rev_backed_base = *ChainstateActive().m_coins_views->m_cacheview->base;
    rev_m_cacheview = std::make_shared<CCoinsViewCache>(CCoinsViewCache(&rev_backed_base));
    rev_m_cacheview->cachedCoinsUsage = ChainstateActive().m_coins_views->m_cacheview->cachedCoinsUsage;
    rev_m_cacheview->hashBlock = ChainstateActive().m_coins_views->m_cacheview->hashBlock;
    for (auto& entry : ChainstateActive().m_coins_views->m_cacheview->cacheCoins) {
        rev_m_cacheview->cacheCoins.insert(entry);
    }
    std::cout << "Rev pool size: " << rev_m_cacheview->cacheCoins.size() << std::endl;

    // step 5 : save mempool
    auto& mp = mempool;
    {
        LOCK(mp.cs);
        rev_nTransactionsUpdated = mp.nTransactionsUpdated.load();
        rev_totalTxSize = mp.totalTxSize;
        // rev_m_total_fee = mp.m_total_fee;
        rev_cachedInnerUsage = mp.cachedInnerUsage;
        rev_lastRollingFreeUpdate = mp.lastRollingFeeUpdate;
        rev_blockSinceLastRollingFeeBump = mp.blockSinceLastRollingFeeBump;
        rev_rollingMinimumFeeRate = mp.rollingMinimumFeeRate;
        // rev_m_epoch = mp.m_epoch;
        // rev_m_sequence_number = mp.m_sequence_number;
        rev_m_is_loaded = mp.m_is_loaded;
        // rev_mapTx = mp.mapTx;
        rev_vTxHashes = mp.vTxHashes;
        // rev_m_unbroadcast_txids = mp.m_unbroadcast_txids;
        rev_mapNextTx = mp.mapNextTx;
        rev_mapDeltas = mp.mapDeltas;
    }

    // step 6 : save global disk file parameters
    rev_vinfoBlockFile = vinfoBlockFile;
    rev_nLastBlockFile = nLastBlockFile;
    rev_setDirtyBlockIndex = setDirtyBlockIndex;
    rev_setDirtyFileInfo = setDirtyFileInfo;
}

const std::string ToString(CValidationState &state)
{
  if (state.IsValid()) return "Valid";
  if (!state.GetDebugMessage().empty())
    return state.GetRejectReason() + ", " + state.GetDebugMessage();
  return state.GetRejectReason();
}

void initialize_block()
{
    // Anonymized_Author_B: For debug purpose
    std::setvbuf(stdout, NULL, _IONBF, 0);

    auto start = std::chrono::high_resolution_clock::now();
    static const ECCVerifyHandle verify_handle;
    SelectParams(CBaseChainParams::REGTEST);

    //  Anonymized_Author_B: Temporarily Re-enabled logging for debugging reasons. We may disable this again to achieve better speed.
    // static const auto testing_setup = MakeNoLogFileContext<const TestingSetup>();
    static const auto testing_setup = MakeUnique<const TestingSetup>(CBaseChainParams::REGTEST);
    g_setup = testing_setup.get();

    int64_t mocktime{1637053432};
    SetMockTime(mocktime);
    
    //  Anonymized_Author_B: Suppose that block chain is mounted at "pathImport"
    //          bitcoin/import, bitcoin/results, bitcoin/corpus
    pathImport = fs::current_path() / "import" / "import.dat";
    {
        LOCK(cs_main);
        std::cout << strprintf ("Importing blocks file %s...\n", pathImport);
    }
    FILE *file = fsbridge::fopen (pathImport, "rb");
    assert(file);

    // Anonymized_Author_B: (1) Load blocks from external file, (2) Link the loaded blocks into best chain
    LoadExternalBlockFile (Params(), file);
    CValidationState state;
    assert (ActivateBestChain(state, Params()));
    // Anonymized_Author_B: We need txInPool updated to get coins used when creating new blocks in our custom mutator.
    CBlockIndex *index;
    WITH_LOCK(cs_main, index = LookupBlockIndex(ChainActive().Tip()->GetBlockHash()));
    CBlock block;
    ReadBlockFromDisk(block, index, Params().GetConsensus());
    while (!block.hashPrevBlock.IsNull())
    {
        COutPoint coin = COutPoint{block.vtx[0]->GetHash(), 0};
        CTxIn tx_in = CTxIn{coin};
        CAmount tx_in_value;
        {
            LOCK(cs_main);
            tx_in_value = ChainstateActive().CoinsTip().AccessCoin(coin).out.nValue;
        }
        txInPool.push_back(std::make_pair(tx_in, tx_in_value));
        WITH_LOCK(cs_main, index = LookupBlockIndex(index->pprev->GetBlockHash()));
        ReadBlockFromDisk(block, index, Params().GetConsensus());
    }
    std::cout << "Tip: " << ChainActive().Tip()->GetBlockHash() << std::endl;
    std::cout << "# of coins: " << txInPool.size() << std::endl;
    std::reverse(txInPool.begin(), txInPool.end());

    pindexFork = ChainActive().Tip();
    assert(pindexFork);
    std::cout << "Tip has work: " << pindexFork->nChainWork.ToString() << std::endl;
    std::cout << "Tip nBits: " << pindexFork->nBits << std::endl;
    arith_uint256 bnTarget;
    bnTarget.SetCompact(pindexFork->nBits);
    std::cout << "Tip Diff: " << bnTarget.ToString() << std::endl;

    assert (txInPool.size() >= COINBASE_MATURITY);
    txInPool.resize(txInPool.size() - COINBASE_MATURITY);
    seenTxIn = txInPool;

    SaveBlockChain();

    assert (txInPool.size());
    {
        LOCK(cs_main);
        CCoinsViewCache& coinsCache = ChainstateActive().CoinsTip();
        CCoinsViewDB& coinsDB = ChainstateActive().CoinsDB();
        CCoinsMap& coinsMap = coinsCache.cacheCoins;
        
        std::unique_ptr<CCoinsViewCursor> pcursor(coinsDB.Cursor());
        assert (pcursor);

        std::cout << "--- DB UTXO ---\n";
        while (pcursor->Valid())
        {
            COutPoint key;
            Coin coin;
            if (pcursor->GetKey(key) && pcursor->GetValue(coin))
                std::cout << "Hash : " << key.hash.GetHex() << std::endl;
            pcursor->Next();
        }
        std::cout << "--- Memory UTXO ---\n";
        for(auto it : coinsMap)
        {
            std::cout << "Hash : " << it.first.hash.GetHex() 
                    << ", flag : " << (it.second.flags & CCoinsCacheEntry::FRESH ? "FRESH" : "NOT FRESH") << ", "
                    << (it.second.flags & CCoinsCacheEntry::DIRTY ? "DIRTY" : "NOT DIRTY") << std::endl;
        }
        std::cout << "--- txInPool ---\n";
        for(auto it : txInPool)
        {
            std::cout << "Prevout: " << it.first.prevout.hash << ", " << it.first.prevout.n << ", " << it.second <<  std::endl;
        }
    }

    hashTip = ChainActive().Tip();

    auto end = std::chrono::high_resolution_clock::now();
    init_time += std::chrono::duration_cast<std::chrono::milliseconds>(end - start);
    std::cout << "Initialization took " << (std::chrono::duration_cast<std::chrono::milliseconds>(init_time)).count() << "ms" << std::endl;
}

void test_one_input(const std::vector<uint8_t> buffer)
{
    if (INITIALIZE(run_count))
      initialize_block();

    // if (RUN_COUNT_REACHED_MAX(run_count)) 
    // {
    //     LOCK(cs_main);
    //     // Anonymized_Author_B: statistics
    //     std::cout << "------------- Statistics -------------\n";
    //     std::cout << "Parsed:\t" << parsed << "\nAccepted:\t" << accepted << std::endl;
    //     std::cout << "Test time:\t" << (std::chrono::duration_cast<std::chrono::milliseconds>(test_time)).count() << std::endl;
    //     std::cout << "Mutation time:\t" << (std::chrono::duration_cast<std::chrono::milliseconds>(mutate_time)).count() << std::endl;

    //     fsbridge::ofstream fileDebug ("time-size.txt");
    //     for (auto& time : vTestTimes)
    //         fileDebug << time << ",";
    //     fileDebug << std::endl;
    //     for (auto& time : vTestTimesAvg)
    //         fileDebug << time << ",";
    //     fileDebug << std::endl;
    //     for (auto& time : vRevTimes)
    //         fileDebug << time << ",";
    //     fileDebug << std::endl;
    //     for (auto& time : vRevTimesAvg)
    //         fileDebug << time << ",";
    //     fileDebug << std::endl;
    //     for (auto& time : vMutTimes)
    //         fileDebug << time << ",";
    //     fileDebug << std::endl;
    //     for (auto& time : vMutTimesAvg)
    //         fileDebug << time << ",";
    //     fileDebug << std::endl;
    //     for (auto& size : vCaseSize)
    //         fileDebug << size << ",";
    // }
    // assert (!RUN_COUNT_REACHED_MAX(run_count));

    run_count++;

    std::cout << "\n--- Execute testing function BEFORE " << run_count << "---\n";
    auto start = std::chrono::high_resolution_clock::now();
    CDataStream ds(buffer, SER_NETWORK, INIT_PROTO_VERSION);
    TestCaseType TestCase = ParseInput ((uint8_t*)ds.data(), ds.size());
    // PrintTestCase (TestCase);
    std::vector <std::vector<CBlockIndex *>> pindexTestCase;

    // SHA1 hash is the name of the generated test case in corpus directory
    std::vector<unsigned char> hashTestCase, hashResultFile;
    hashTestCase.resize (CSHA1::OUTPUT_SIZE);
    CSHA1().Write ((const unsigned char*)(ds.data()), ds.size()).Finalize(hashTestCase.data());

    // fs::path pathBefore = fs::current_path() / "debug" / PACKAGE_VERSION / "rs-before.out";
    // fsbridge::ofstream fileBefore(pathBefore, std::ios_base::out | std::ios_base::app);
    // for (auto ch : HexStr(hashTestCase))
    //     fileBefore << ch;
    // fileBefore << "\n";
    // fileBefore.close();
    std::cout << "\tTest case hash: " << HexStr(hashTestCase) << "\n";

    if (TestCase.empty())
    {
        std::cout << "\tParse Failure\n";
        return;
    } parsed++;

    json result;
    int nAccept = 0, nTotal = 0, nBlock = 0;
    for (auto& block : TestCase)
    {
        CValidationState state;
        auto key = "Block " + padStringWithZeros(std::to_string(nBlock), 2);
        bool fFirstSeen{false}, fAccepted{false};
        bool fProcessed = FuzzProcessNewBlock (state, Params(), std::make_shared<CBlock>(block.block), true, &fFirstSeen);
        // std::cout << "\t\tFuzzProcessNewBlock: " << std::boolalpha << fProcessed << (fFirstSeen ? ", new block" : ", not new block") << std::endl;
        // fileOutput << strprintf ("%s; %s; %s; %s\n", block.GetHash(), fAccepted ? "true" : "false", state_validity.ToString(), HexStr(vchHash));
        {
            LOCK(cs_main);
            auto checkblockPass = BlockIndex().find(block.block.GetHash());
            if(checkblockPass != BlockIndex().end()) {
                CBlockIndex* pindex = checkblockPass->second;
                // std::cout << "\t\tBlock has work: " << GetBlockProof(*pindex).ToString() << std::endl;
                // std::cout << "\t\tNew chainwork: " << pindex->nChainWork.ToString() << std::endl;
                if (pindex->IsValid(BLOCK_VALID_SCRIPTS)) {
                    fAccepted = true;
                    nAccept++;
                }
            }
        }

        result[key]["accept"] = (fProcessed && fAccepted) ? true : false;
        result[key]["reason"] = ToString(state);

        nTotal++;
        nBlock++;
    }
    result["HashTip"] = ChainActive().Tip()->GetBlockHash().ToString();
    
    // Anonymized_Author_B: Print blockchain state (UTXOs)
    if (PRINT_UTXO)
    {
        result["UTXO"] = nullptr;
        LOCK(cs_main);
        CCoinsViewCache& coinsCache = ChainstateActive().CoinsTip(); 
        CCoinsMap& coinsMap = coinsCache.cacheCoins;
        for(auto it : coinsMap)
        {
            result["UTXO"][it.first.hash.GetHex()] = nullptr;
            result["UTXO"][it.first.hash.GetHex()]["FRESH"] = (it.second.flags & CCoinsCacheEntry::FRESH ? true : false);
            result["UTXO"][it.first.hash.GetHex()]["DIRTY"] = (it.second.flags & CCoinsCacheEntry::DIRTY ? true : false);
        }
    }
    auto packed = json::to_msgpack(result);
    hashResultFile.resize (CSHA1::OUTPUT_SIZE);
    CSHA1().Write ((const unsigned char*)(packed.data()), packed.size()).Finalize(hashResultFile.data());

    fs::path pathResult = fs::current_path() / "results" / PACKAGE_VERSION / (HexStr(hashTestCase) + "_" + HexStr(hashResultFile));
    fsbridge::ofstream fileResult(pathResult, std::ios_base::out);
    fileResult << std::setw(4) << result;
    fileResult.close();

    // fs::path pathDebug = fs::current_path() / "debug" / PACKAGE_VERSION / "rs-after.out";
    // fsbridge::ofstream fileDebug(pathDebug, std::ios_base::out | std::ios_base::app);
    // for (auto ch : HexStr(hashTestCase))
    //     fileDebug << ch;
    // fileDebug << "\n";
    // fileDebug.close();

    {
        LOCK(cs_main);
        std::cout << "\tNew tip after processing: " ;
        CBlockIndex* tip = ChainActive().Tip();
        strprintf("Chainstate [%s] @ height %d (%s)\n", "ibd",
         tip ? tip->nHeight : -1, tip ? tip->GetBlockHash().ToString() : "null");
    }

    // Anonymized_Author_B: Print blockchain state (UTXOs)
    if (false)
    {
        LOCK(cs_main);
        CCoinsViewCache& coinsCache = ::ChainstateActive().CoinsTip();
        CCoinsViewDB& coinsDB = ChainstateActive().CoinsDB();
        CCoinsMap& coinsMap = coinsCache.cacheCoins;
        
        std::unique_ptr<CCoinsViewCursor> pcursor(coinsDB.Cursor());
        assert (pcursor);
        std::cout << "--- DB UTXO ---\n";
        while (pcursor->Valid())
        {
            COutPoint key;
            Coin coin;
            if (pcursor->GetKey(key) && pcursor->GetValue(coin))
                std::cout << "Hash : " << key.hash.GetHex() << std::endl;
            pcursor->Next();
        }
        std::cout << "--- Memory UTXO ---\n";
        for(auto it : coinsMap)
        {
            std::cout << "Hash : " << it.first.hash.GetHex() 
                    << ", flag : " << (it.second.flags & CCoinsCacheEntry::FRESH ? "FRESH" : "NOT FRESH") << ", "
                    << (it.second.flags & CCoinsCacheEntry::DIRTY ? "DIRTY" : "NOT DIRTY") << std::endl;
        }
    }

    // Anonymized_Author_B: revert the blockchain and blockchain state
    // step 1 : Disconnect accepted blocks from mempool and UTXO cache
    std::cout << "\t\tTotal " << nAccept << " blocks accepted\n";
    auto revStart = std::chrono::high_resolution_clock::now();
    auto& activeChainState = ChainstateActive();
    auto& blockman = activeChainState.m_blockman;
    // while (nAccept > 0)
    // {
    //     {
    //         LOCK(cs_main);
    //         LOCK(mempool.cs);
    //         CValidationState state;
    //         DisconnectedBlockTransactions disconnectedpool;
    //         activeChainState.DisconnectTip(state, Params(), &disconnectedpool);
    //         UpdateMempoolForReorg(disconnectedpool, true);
    //     }
    //     CValidationState state;
    //     activeChainState.ActivateBestChain(state, Params(), nullptr);
    //     nAccept--;
    // }

    // step 2-1 : revert ALL validation.cpp global paremeters
    pindexBestHeader = rev_pindexBestHeader;
    g_best_block = rev_g_best_block;
    nMaxTipAge = rev_nMaxTipAge;
    hashAssumeValid = rev_hashAssumeValid;
    nMinimumChainWork = rev_nMinimumChainWork;
    minRelayTxFee = rev_minRelayTxFee;
    pindexBestInvalid = rev_pindexBestInvalid;

    // step 3 : revert chain state
    activeChainState.m_chain.vChain = rev_m_chain;
    activeChainState.nBlockSequenceId = rev_nBlockSequenceId;
    activeChainState.nBlockReverseSequenceId = rev_nBlockReverseSequenceId;
    activeChainState.nLastPreciousChainwork = rev_nLastPreciousChainwork;
    activeChainState.m_cached_finished_ibd = rev_m_cached_finished_ibd.load();
    // activeChainState.m_coinsdb_cache_size_bytes = rev_m_coinsdb_cache_size_bytes;
    // activeChainState.m_coinstip_cache_size_bytes = rev_m_coinstip_cache_size_bytes;
    activeChainState.setBlockIndexCandidates = rev_setBlockIndexCandidates;

    // step 4 : revert block manager
    blockman.m_failed_blocks = rev_m_failed_blocks;
    blockman.m_blocks_unlinked = rev_m_blocks_unlinked;

    // step 5 : revert UTXO cache
    {
        LOCK(cs_main);
        auto& cv = activeChainState.m_coins_views->m_cacheview;
        cv->hashBlock = rev_m_cacheview->hashBlock;
        cv->cachedCoinsUsage = rev_m_cacheview->cachedCoinsUsage;
        // cv->cacheCoins = rev_m_cacheview->cacheCoins;
        cv->cacheCoins.clear();
        for (auto& entry : rev_m_cacheview->cacheCoins)
            cv->cacheCoins.insert(entry);
    }

    // step 6 : revert mempool
    auto& mp = mempool;
    {
        LOCK(mp.cs);
        mp.nTransactionsUpdated = rev_nTransactionsUpdated.load();
        mp.totalTxSize = rev_totalTxSize;
        // mp.m_total_fee = rev_m_total_fee;
        mp.cachedInnerUsage = rev_cachedInnerUsage;
        mp.lastRollingFeeUpdate = rev_lastRollingFreeUpdate;
        mp.blockSinceLastRollingFeeBump = rev_blockSinceLastRollingFeeBump;
        mp.rollingMinimumFeeRate = rev_rollingMinimumFeeRate;
        // mp.m_epoch = rev_m_epoch;
        // mp.m_sequence_number = mp.m_sequence_number;
        mp.m_is_loaded = rev_m_is_loaded;
        // mp.mapTx = rev_mapTx;
        mp.vTxHashes = rev_vTxHashes;
        // mp.m_unbroadcast_txids = rev_m_unbroadcast_txids;
        mp.mapNextTx = rev_mapNextTx;
        mp.mapDeltas = rev_mapDeltas;
    }

    // step 2 : Delete new blocks from block indexes
    // If a block index is not in rev_BlockIndexes, delete
    // If it is, overwrite the old block index (for disk file position)
    {
        LOCK(cs_main);
        std::set<uint256> delete_keys;
        for (const BlockMap::value_type& entry : blockman.m_block_index) {
            auto it = rev_BlockIndexes.find(entry.first);
            // If the block index points the input blockchain
            if (it != rev_BlockIndexes.end()) {
                *(entry.second) = it->second;
            }
            // Else
            else {
                delete entry.second;
                delete_keys.insert(entry.first);
            }
        }
        // rev_BlockIndexes now has 1) the initial block index + 2) the unwanted block indexes, but their memory freed.
        for (auto& key : delete_keys)
            blockman.m_block_index.erase(blockman.m_block_index.find(key));
    }

    // step 7 : revert global disk file parameters
    vinfoBlockFile = rev_vinfoBlockFile;
    nLastBlockFile = rev_nLastBlockFile;
    setDirtyBlockIndex = rev_setDirtyBlockIndex;
    setDirtyFileInfo = rev_setDirtyFileInfo;

    std::cout << "\t\tTry ActivateBestChain() on reverted chain state\n";
    CValidationState state;
    activeChainState.ActivateBestChain(state, Params(), nullptr);

    {
        LOCK(cs_main);
        std::cout << "\tNew tip after processing: ";
        CBlockIndex* tip = ChainActive().Tip();
        strprintf("Chainstate [%s] @ height %d (%s)\n", "ibd",
         tip ? tip->nHeight : -1, tip ? tip->GetBlockHash().ToString() : "null");
    }
    
    auto revEnd = std::chrono::high_resolution_clock::now();
    rev_time += std::chrono::duration_cast<std::chrono::milliseconds>(revEnd - revStart);
    std::cout << "\tRevert time: " << (std::chrono::duration_cast<std::chrono::milliseconds>(revEnd - revStart)).count() << "ms" << std::endl;
    std::cout << "\tRevert time avg: " << (std::chrono::duration_cast<std::chrono::milliseconds>(rev_time)).count() / run_count << "ms" << std::endl;
    vRevTimes.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(revEnd - revStart)).count());
    vRevTimesAvg.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(rev_time)).count() / run_count);

    std::cout << std::endl;
    auto end = std::chrono::high_resolution_clock::now();
    test_time += std::chrono::duration_cast<std::chrono::milliseconds>(end - start);
    std::cout << "\tTest time: " << (std::chrono::duration_cast<std::chrono::milliseconds>(end - start)).count() << "ms" << std::endl;
    std::cout << "\tTest time avg: " << (std::chrono::duration_cast<std::chrono::milliseconds>(test_time)).count() / run_count << "ms" << std::endl;
    vTestTimes.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(end - start)).count());
    vTestTimesAvg.push_back((std::chrono::duration_cast<std::chrono::milliseconds>(test_time)).count() / run_count);
    vCaseSize.push_back(nTotal);

    // WriteTestCase (buffer.data(), buffer.size());
}

// Forward-declare the libFuzzer's mutator callback.
extern "C" size_t
LLVMFuzzerMutate(uint8_t *Data, size_t Size, size_t MaxSize);

extern "C" size_t LLVMFuzzerCustomMutator(uint8_t *Data, size_t Size,
                                        size_t MaxSize, unsigned int Seed) {
    return __CustomMutator(Data, Size, MaxSize, Seed);
}
