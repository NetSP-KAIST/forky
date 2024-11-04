#!/bin/bash

CURR=$PWD
cd workspace
WORKDIR=$PWD
echo $1

mkdir -p $WORKDIR/old_results/old$1
mkdir -p $WORKDIR/old_results/old$1/test_cases_with_check
mkdir -p $WORKDIR/old_results/old$1/test_cases
mkdir -p $WORKDIR/old_results/old$1/blocks
mkdir -p $WORKDIR/old_results/old$1/attestations
mkdir -p $WORKDIR/old_results/old$1/corpus
mkdir -p $WORKDIR/old_results/old$1/anchor_block
mkdir -p $WORKDIR/old_results/old$1/anchor_state

mv $WORKDIR/reports/new_reorg.txt $WORKDIR/old_results/old$1
mv $WORKDIR/reports/reorg_count.txt $WORKDIR/old_results/old$1
mv $WORKDIR/test_cases/* $WORKDIR/old_results/old$1/test_cases
mv $WORKDIR/test_cases_with_check/* $WORKDIR/old_results/old$1/test_cases_with_check 
mv $WORKDIR/corpus/* $WORKDIR/old_results/old$1/corpus
mv $WORKDIR/blocks/* $WORKDIR/old_results/old$1/blocks
mv $WORKDIR/attestations/* $WORKDIR/old_results/old$1/attestations
cp $WORKDIR/anchor_block/* $WORKDIR/old_results/old$1/anchor_block
cp $WORKDIR/anchor_state/* $WORKDIR/old_results/old$1/anchor_state

mv $CURR/debug1.log $WORKDIR/old_results/old$1
mv $CURR/debug2.log $WORKDIR/old_results/old$1