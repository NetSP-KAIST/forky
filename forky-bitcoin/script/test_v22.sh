#!/bin/bash
cd /bitcoin
echo "start Testing"
# for FILE in ./test_cases/*; 
# do 
#     sh -c "FUZZ=block src/test/fuzz/fuzz $FILE -detect_leaks=0"; 
# done

#find ./test_cases -name "*" -exec sh -c \"FUZZ=block src/test/fuzz/fuzz {} -detect_leaks=0\" \;

#hex=$(printf "%02x " {0..255})
# hex=$(printf "%01x " {0..15})
# for FILES in $hex
# do
#    FUZZ=block src/test/fuzz/fuzz test_cases/$FILES* -detect_leaks=0
# done

FUZZ=block src/test/fuzz/fuzz test_cases/ -max_len=4000000 -runs=0