#!/bin/bash

WORKDIR=$PWD

rm -rf ./lighthouse/testing/ef_tests/forky/pyspec_tests
mkdir -p ./lighthouse/testing/ef_tests/forky
cp -r ./testcases ./lighthouse/testing/ef_tests/forky/
mv ./lighthouse/testing/ef_tests/forky/testcases ./lighthouse/testing/ef_tests/forky/pyspec_tests
cd ./lighthouse
echo "START Lighthouse Test"
cargo test --release -p ef_tests --features "ef_tests" | tee $WORKDIR/logs_lighthouse
echo "END Lighthouse Test"
echo "END Lighthouse Test" >> $WORKDIR/logs_lighthouse