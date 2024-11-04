pub mod epoch;
pub mod branch;
pub mod block;
pub mod attestation;
pub mod timing;
pub mod utils;
pub mod tester;
extern crate rand;
use rand::{rngs::StdRng, Rng, thread_rng};
use rand::prelude::*;
use rand::distributions::WeightedIndex;
use tree_hash::TreeHash;
use targets::fork_choice::Step;
use futures::executor::block_on;
use snap::write::FrameEncoder;

use types::{
    BeaconState, SignedBeaconBlock, EthSpec, MainnetEthSpec, ForkName
};

use std::fs::File;
use std::io::{BufWriter, Write};
use ssz::{Encode, ssz_encode};


pub fn mutate_branch(steps: &mut Vec<Step<String, String, String, String>>, 
    rng: &StdRng, 
    size: usize, 
    max_size: usize) -> Vec<Step<String, String, String, String>> {
    // Empty function
    println!("[Topology Mutation]");

    let choices = [0, 1, 2, 3, 4];
    let weights = [1, 3, 2, 3, 1];
    let dist = WeightedIndex::new(&weights).unwrap();
    let mut rng = thread_rng();
    match choices[dist.sample(&mut rng)] {
        // Delete random leaf block and following attestations
        0 => block::delete_block(steps.clone()),
        // Add child block to the longest leaf block
        1 => branch::depth_first_add(steps.clone()),
        // Add child block to a random non-leaf block
        2 => branch::breadth_first_add(steps.clone()),
        // Add child block to the shortest leaf block
        3 => branch::balance_first_add(steps.clone()),
        // Keep current topology and do "Vote mutation"
        4 => branch::keep_topology(steps.clone()),
        _ => unreachable!(),
    }
    // branch::breadth_first_add(steps.clone())
}

// Deprecated
pub fn mutate_block(steps: &mut Vec<Step<String, String, String, String>>, 
    rng: &StdRng, 
    size: usize, 
    max_size: usize) -> Vec<Step<String, String, String, String>> {
    // Empty function
    println!("[Block Mutation]");
    // get random leaf block and make it invalid
    steps.clone()
}

pub fn mutate_attestation(steps: &mut Vec<Step<String, String, String, String>>, 
    rng: &StdRng, 
    size: usize, 
    max_size: usize) -> Vec<Step<String, String, String, String>> {
    // Sometimes it will attest to very old block
    attestation::add_random_attestation(steps.clone())
}


// Note: The cost of Epoch mutation is extremely high and make test cases huge.
pub fn mutate_epoch(steps: &mut Vec<Step<String, String, String, String>>, 
    rng: &StdRng, 
    size: usize, 
    max_size: usize) -> Vec<Step<String, String, String, String>> {
    // Empty function

    // TODO: Check epoach addable:
    // 1. Check n_blocks in steps?  limit = 144: 4.5 epoch
    let mut chain = utils::Chain::new();
    let mut n_blocks = 0;
    for step in steps.iter() {
        match step {
            Step::Block{ block } => {
                let _block = utils::get_block_with_string(block.clone());
                n_blocks += 1;
                let post_state = chain.get_poststate(_block.clone());
                chain.insert_block_to_chain(_block.clone(), post_state);
            },
            _ => (),
        }
    }
    if n_blocks > 144 {
        println!("[Epoch Mutation] SKIP: n_blocks > 144");
        return steps.clone();
    }

    // 2. Check finalized is changed?
    let leaves = chain.get_leaf_blocks();
    for leaf in leaves{
        println!("[Epoch Mutation] leaf.post_state.finalized_checkpoint().epoch: {:?}", leaf.post_state.finalized_checkpoint().epoch);
        println!("[Epoch Mutation] leaf.post_state.justified_checkpoint().epoch: {:?}", leaf.post_state.current_justified_checkpoint().epoch);
        if leaf.post_state.finalized_checkpoint().epoch > 0 {
            println!("[Epoch Mutation] SKIP: finalized is changed");
            return steps.clone();
        }
        // println!("leaf.post_state.finalized_checkpoint.epoch: {:?}", leaf.post_state.finalized_checkpoint());
    }

    println!("[Epoch Mutation] n_blocks: {:?}", n_blocks);
    let choices = [0, 1, 2, 3, 4];
    let weights = [5, 1, 30, 1, 15];
    let dist = WeightedIndex::new(&weights).unwrap();
    let mut rng = thread_rng();
    match choices[dist.sample(&mut rng)] {
        // skip current epoch
        0 => epoch::next_epoch(steps.clone()),
        // Add 32 blocks (empty attestations)
        1 => epoch::add_full_epoch_with_skip_left_slots(steps.clone()),
        // Add 22+ attested blocks, justify previous checkpoint
        2 => epoch::add_justifiable_epoch_with_skip_left_slots(steps.clone()),
        // Add 32 blocks without attestations
        3 => epoch::add_unjustifiable_epoch_with_skip_left_slots(steps.clone()),
        // Add 2 epoch: Super high cost!!!
        4 => epoch::finalize_current_epoch(steps.clone()),
        _ => unreachable!(),
    }
    // epoch::add_justifiable_epoch_with_skip_left_slots(steps.clone())
}

pub fn mutate_timing(steps: &mut Vec<Step<String, String, String, String>>, 
    rng: &StdRng, 
    size: usize, 
    max_size: usize) -> Vec<Step<String, String, String, String>> {
    timing::shuffle_tick_within_slot(steps.clone())
}