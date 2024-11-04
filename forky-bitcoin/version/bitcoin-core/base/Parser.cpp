#include <cassert>
#include <cstdlib>
#include <cstdint>
#include <cstring>
#include <iostream>

#include <vector>
#if defined(HAVE_CONFIG_H)
#include <config/bitcoin-config.h>
#endif

#include <primitives/block.h>
#include <streams.h>
#if CLIENT_VERSION_MAJOR <= 26
#include <version.h>
#else
#include <clientversion.h>
#endif
#include <crypto/sha1.h>
#include <consensus/merkle.h>

#include <test/fuzz/FuzzedDataProvider.h>
#include "Parser.h"
#include <bitset>
#include <iomanip>

#define TRUE 0xAA
#define FALSE 0xBB

std::vector<std::vector<uint8_t>> SplitInput(const uint8_t *Data, size_t Size,
                                     const uint8_t *Separator,
                                     size_t SeparatorSize) {
  std::vector<std::vector<uint8_t>> Res;
  assert(SeparatorSize > 0);
  auto Beg = Data;
  auto End = Data + Size;
  while (const uint8_t *Pos = (const uint8_t *)memmem(Beg, End - Beg,
                                     Separator, SeparatorSize)) {
    Res.push_back(std::vector<uint8_t>(Beg, Pos));
    Beg = Pos + SeparatorSize;
  }
  if (Beg < End)
    Res.push_back(std::vector<uint8_t>(Beg, End));
  return Res;
}

TestCaseType ParseInput (uint8_t *Data, size_t Size)
{
    if (Data == nullptr) {
        std::cout << "\tData is nullptr\n";
        return TestCaseType();
    }
    TestCaseType TestCase;

    const uint8_t SepBlock[] = {0xFA, 0xBF, 0xB5, 0xDA};

    auto blocks = SplitInput(Data, Size, SepBlock, sizeof(SepBlock));
    std::cout << "Unserializing " << blocks.size() << " blocks ...\n";
    for (auto& block : blocks)
    {
        std::cout << "TestCaseBlock has size: " << block.size() << std::endl;
        CBlock blk;
        bool fValid{false};
        int nChild, prev;

        if (block.size() <= 9) {
            std::cout << "\tData not in format\n";
            return TestCaseType();
        }

#if CLIENT_VERSION_MAJOR <= 26
        CDataStream ds(block, SER_NETWORK, INIT_PROTO_VERSION);
#else
        DataStream ds{block};
#endif
        try {
            ds >> fValid;
            ds >> nChild;
            ds >> prev; // 4 Bytes
#if CLIENT_VERSION_MAJOR <= 26
            ds >> blk;
#else
            ds >> TX_WITH_WITNESS(blk);
#endif
        } catch (const std::ios_base::failure& e) {
            // Parse Failure
            std::cout << "\tParseInput() I/O failure: \n" << e.code() << ", " << e.what() << "\n";
            return TestCaseType();
        }
        TestCase.push_back(TestCaseBlock(blk, fValid, nChild, prev));
    }

    return TestCase;
}

std::vector <uint8_t> SerializeTestCase (TestCaseType& TestCase)
{
    std::cout << "Serializing " << TestCase.size() << " blocks ...\n";
    std::vector <uint8_t> Serialized = std::vector <uint8_t>();

    if (TestCase.empty()) {
        Serialized.push_back(0);
        return Serialized;
    }

    for (auto& block : TestCase)
    {
#if CLIENT_VERSION_MAJOR <= 26
        CDataStream ds(SER_NETWORK, INIT_PROTO_VERSION);
#else
        DataStream ds;
#endif
        ds << block.isValid;
        ds << block.nChild;
        ds << block.prev;
#if CLIENT_VERSION_MAJOR <= 26
        ds << block.block;
#else
        ds << TX_WITH_WITNESS(block.block);
#endif
        for (auto& byte : ds)
            Serialized.push_back ((unsigned char)byte);

        Serialized.push_back(0xFA); Serialized.push_back(0xBF); Serialized.push_back(0xB5); Serialized.push_back(0xDA);
    }
    Serialized.pop_back(); Serialized.pop_back(); Serialized.pop_back(); Serialized.pop_back();
    return Serialized;
}

void PrintTestCase (TestCaseType& TestCase, PrintUpTo level)
{
    return;
    int nBranch = 1, nBlock = 1, nTx = 1;
    for (auto& block : TestCase) {
        std::cout << "\tBlock " << nBlock << ": " << block.block.GetHash().ToString() << "\n";
        std::cout << "\t\tPrevBlock: " << block.block.hashPrevBlock.ToString() << "\n";
        std::cout << "\t\tMerkleRoot: " << block.block.hashMerkleRoot.ToString() << "\n";
        std::cout << "\t\t(Correct) mkrl: " << BlockMerkleRoot(block.block).ToString() << "\n";
        std::cout << "\t\tnTime: " << block.block.nTime << "\n";
        std::cout << "\t\tnVersion: " << block.block.nVersion << "\n";
        std::cout << "\t\tnBits: " << block.block.nBits << "\n";
        std::cout << "\t\tNonce: " << block.block.nNonce << "\n";
        std::cout << std::boolalpha<< "\t\tValid: " << block.isValid << ", nChild: " << block.nChild << ", Prev: " << block.prev << std::endl;
        nBlock ++;
        
        if (level == PrintUpTo::UPTO_METADATA) continue;

        nTx = 1;
        for (auto& txref : block.block.vtx) {
            std::cout << "\t\tTx" << nTx << ": " << txref->GetHash().ToString() << "\n";
            nTx++;
            if (level == PrintUpTo::UPTO_TX) continue;

            std::cout << "\t\t\tvin size: " << txref->vin.size() << ", vout size: " << txref->vout.size() << std::endl;
            for (auto& txin : txref->vin) {
                std::cout << "\t\t\tin: " << txin.ToString() << std::endl;
            }
            for (auto& txout : txref->vout) {
                std::cout << "\t\t\tout: " << txout.ToString() << std::endl;
            }
        }
    }
}

int GetNumberOfBlocks (TestCaseType& TestCase)
{
    return TestCase.size();
}

std::string padStringWithZeros(const std::string& inputString, int desiredLength) {
    if (inputString.length() >= desiredLength) {
        return inputString; // No need to pad if the string is already as long as desired.
    }

    int zerosToPad = desiredLength - inputString.length();
    std::string paddedString(zerosToPad, '0'); // Create a string of zeros of the required length
    paddedString += inputString; // Concatenate the inputString after the zeros

    return paddedString;
}