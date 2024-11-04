#!/bin/bash
cd /bitcoin
#mv /bitcoin/src/test/fuzz/block.cpp /bitcoin/src/test/fuzz/block.cpp.old
#cp /code/block.cpp /bitcoin/src/test/fuzz/block.cpp 
#RUN CC=/usr/bin/clang-13 CXX=/usr/bin/clang++-13 ./configure --enable-fuzz --with-sanitizers=address,fuzzer,undefined --disable-wallet --without-gui
#make -j 8
# chmod 755 /bitcoin/import/import.dat
# mv /bitcoin/import/import.dat /bitcoin/import/import.dat.old
# cp /bitcoin/import/import.dat.old /bitcoin/import/import.dat
# mkdir /bitcoin/test_cases/22.0.0
echo "start fuzzing"
FUZZ=block src/test/fuzz/fuzz fuzzing_corpus/ -max_len=4000000 -rss_limit_mb=0 -runs=500
#FUZZ=block src/test/fuzz/fuzz fuzzing_corpus