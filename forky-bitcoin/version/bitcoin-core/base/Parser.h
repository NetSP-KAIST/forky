#ifndef PARSER_H
#define PARSER_H

#include <vector>
#include <iostream>
#include <string>

#include <boost/multi_index_container.hpp>
#include <boost/multi_index/hashed_index.hpp>
#include <boost/multi_index/ordered_index.hpp>
#include <boost/multi_index/sequenced_index.hpp>

class TestCaseBlock
{
public:
    CBlock block;
    bool isValid{false};
    int nChild;
    int prev;

    TestCaseBlock(CBlock &mem, bool valid, int nChild, int prev) : 
        block(mem),
        isValid(valid), nChild(nChild),
        prev(prev) { }
public:
    bool isLeaf() { return nChild <= 0; }
};

enum PrintUpTo : int16_t {
    UPTO_METADATA = std::numeric_limits<int16_t>::min(),
    UPTO_TX,
    UPTO_COIN,
};

typedef std::vector <TestCaseBlock> TestCaseType;

std::vector<std::vector<uint8_t>> SplitInput(const uint8_t *Data, size_t Size,
                                     const uint8_t *Separator,
                                     size_t SeparatorSize);

TestCaseType ParseInput (uint8_t *Data, size_t Size);
std::vector <uint8_t> SerializeTestCase (TestCaseType& TestCase);
void PrintTestCase (TestCaseType& TestCase, PrintUpTo level = PrintUpTo::UPTO_METADATA);
int GetNumberOfBlocks (TestCaseType& TestCase);

std::string padStringWithZeros(const std::string& inputString, int desiredLength);

#endif