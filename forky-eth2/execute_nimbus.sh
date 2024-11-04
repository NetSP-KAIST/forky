#!/bin/bash

WORKDIR=$PWD

mkdir -p ./nimbus-23.5.1/vendor/nim-eth2-scenarios/tests-v1.3.0/mainnet/capella/fork_choice/forky/pyspec_tests
rm -rf ./nimbus-23.5.1/vendor/nim-eth2-scenarios/tests-v1.3.0/mainnet/capella/fork_choice/forky/pyspec_tests
cp -r ./testcases ./nimbus-23.5.1/vendor/nim-eth2-scenarios/tests-v1.3.0/mainnet/capella/fork_choice/forky/
mv ./nimbus-23.5.1/vendor/nim-eth2-scenarios/tests-v1.3.0/mainnet/capella/fork_choice/forky/testcases ./nimbus-23.5.1/vendor/nim-eth2-scenarios/tests-v1.3.0/mainnet/capella/fork_choice/forky/pyspec_tests

cd ./nimbus-23.5.1
echo "START Nimbus Test"
make test 2>&1 | tee $WORKDIR/logs_nimbus
echo "END Nimbus Test"
echo "END Nimbus Test" >> $WORKDIR/logs_nimbus