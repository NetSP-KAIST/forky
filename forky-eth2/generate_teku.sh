#!/bin/bash

WORKDIR=$PWD

testcases=./testcases/*

rm -rf ./teku-23.6.2/eth-reference-tests/src/referenceTest/generated_tests/tech/pegasys/teku/reference/capella/forky
rm -rf ./teku-23.6.2/eth-reference-tests/src/referenceTest/resources/consensus-spec-tests/tests/mainnet/capella/forky
mkdir -p ./teku-23.6.2/eth-reference-tests/src/referenceTest/generated_tests/tech/pegasys/teku/reference/capella/forky
mkdir -p ./teku-23.6.2/eth-reference-tests/src/referenceTest/resources/consensus-spec-tests/tests/mainnet/capella/forky

for testcase in $testcases; do
    testname=$(basename $testcase)
    cp -R $testcase/. ./teku-23.6.2/eth-reference-tests/src/referenceTest/resources/consensus-spec-tests/tests/mainnet/capella/forky/Testcase$testname
    cp ./teku-23.6.2/eth-reference-tests/src/referenceTest/Forky.java ./teku-23.6.2/eth-reference-tests/src/referenceTest/generated_tests/tech/pegasys/teku/reference/capella/forky/Testcase$testname.java
    sed -i "s/PLACEHOLDER/Testcase$testname/g" ./teku-23.6.2/eth-reference-tests/src/referenceTest/generated_tests/tech/pegasys/teku/reference/capella/forky/Testcase$testname.java
done

cd ./teku-23.6.2/eth-reference-tests
echo "START Teku Test"
gradle referenceTest
echo "END Teku Test"