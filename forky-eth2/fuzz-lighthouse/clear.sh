#!/bin/bash
cd ./workspace/
rm -rf ./blocks/*
rm -rf ./attestations/*
rm -rf ./corpus/*
rm -rf ./test_cases/*
rm -rf ./test_cases_with_check/*
rm -rf ./reports/*
mkdir -p ./test_cases
mkdir -p ./test_cases_with_check
cp ./saved_for_clear/block_* ./blocks
cp ./saved_for_clear/attestation_* ./attestations
cp ./saved_for_clear/*.yaml* ./corpus
touch ./reports/reorg_count.txt
echo "reorg" >> ./reports/reorg_count.txt
touch ./reports/new_reorg.txt
echo "timestamp,iter,n_new_reorg,n_reorg_in_tc,n_replacing_blocks,n_replaced_blocks,replacing_slot_distance,replaced_slot_distance,weight_gap_exists,is_boosted,epoch_gap,justified_epoch_gap,n_th_reorg" >> ./reports/new_reorg.txt
