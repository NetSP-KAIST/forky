#!/bin/bash

N_GROUPS=8
WORKDIR=$PWD

for i in $(seq 0 $((N_GROUPS - 1))); do
  cd $WORKDIR 
  echo "START GROUP $i"
  source_directory="$PWD/testcases_splitted/group_$i"
  testcases=$source_directory/*

  # Clean up the target directories
  rm -rf $WORKDIR/teku-23.6.2/eth-reference-tests/src/referenceTest/generated_tests/tech/pegasys/teku/reference/capella/forky
  rm -rf $WORKDIR/teku-23.6.2/eth-reference-tests/src/referenceTest/resources/consensus-spec-tests/tests/mainnet/capella/forky
  mkdir -p $WORKDIR/teku-23.6.2/eth-reference-tests/src/referenceTest/generated_tests/tech/pegasys/teku/reference/capella/forky
  mkdir -p $WORKDIR/teku-23.6.2/eth-reference-tests/src/referenceTest/resources/consensus-spec-tests/tests/mainnet/capella/forky

  for testcase in $testcases; do
    testname=$(basename $testcase)
    cp -R $testcase/. $WORKDIR/teku-23.6.2/eth-reference-tests/src/referenceTest/resources/consensus-spec-tests/tests/mainnet/capella/forky/Testcase$testname
    cp $WORKDIR/teku-23.6.2/eth-reference-tests/src/referenceTest/Forky.java $WORKDIR/teku-23.6.2/eth-reference-tests/src/referenceTest/generated_tests/tech/pegasys/teku/reference/capella/forky/Testcase$testname.java
    sed -i "s/PLACEHOLDER/Testcase$testname/g" $WORKDIR/teku-23.6.2/eth-reference-tests/src/referenceTest/generated_tests/tech/pegasys/teku/reference/capella/forky/Testcase$testname.java
  done
  # mv $WORKDIR/splited/$source_directory $WORKDIR/testcases
  echo "START Teku Test for GROUP $i"
  cd $WORKDIR/teku-23.6.2/eth-reference-tests
  gradle referenceTest

  python3 get_fails.py > $WORKDIR/fails$1.txt
  echo "END Teku Test for GROUP $i"
done

