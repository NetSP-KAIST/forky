use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use types::{Hash256, BeaconState, BeaconBlock, SignedBeaconBlock, 
    BeaconBlockCapella, Slot, BeaconBlockBodyCapella, EthSpec, 
    MainnetEthSpec, ChainSpec, ForkName, Attestation, RelativeEpoch, 
    test_utils::{SeedableRng, TestRandom, XorShiftRng}};
// use crate::test_utils::TestRandom;
use test_random_derive::TestRandom;
use tree_hash::TreeHash;
use ssz::{Encode, ssz_encode};
use std::fs::{File};
use std::io;
use std::io::{BufWriter, Write, Read};
use snap::write::FrameEncoder;
use crate::rand::{Rng, thread_rng};
use rand::prelude::*;
use rand::distributions::WeightedIndex;

pub use types::test_utils::generate_deterministic_keypairs;
use eth2_interop_keypairs::{be_private_key, keypair};

use targets::fork_choice::Step;
use crate::fork_choice_mutator::utils;
use crate::fork_choice_mutator::tester;
use futures::executor::block_on;

use crate::fork_choice_mutator::utils::{BlockNode, Chain};
use std::borrow::Cow;

pub fn vote_mutator(
    steps: Vec<Step<String, String, String, String>>,
    // steps_for_branch: Vec<Step<String, String, String, String>>,
    // pre_blocks: Vec<SignedBeaconBlock<MainnetEthSpec>>,
    target: SignedBeaconBlock<MainnetEthSpec>, 
    target_node: utils::BlockNode,
    chain: utils::Chain,
    current_slot: Slot) 
-> Vec<Step<String, String, String, String>> {
    let choices = [true, false];
    let weights = [2, 1];
    let dist = WeightedIndex::new(&weights).unwrap();
    let mut rng = rand::thread_rng();

    // Step 1: Do or not (Do: 66%, Not: 33%)
    if choices[dist.sample(&mut rng)] {
        // Do Vote Mutation (66%)
        println!("[Vote Mutation]");
        // let mut new_steps: Vec<Step<String, String, String, String>> = steps.clone();
        let head = chain.get_canonical_leaf_node().unwrap();
        let canonical_branch = chain.get_branch(head.clone().block.canonical_root());
        let slot = current_slot + 1;
        
        // Past Epoch Check
        if (target.slot() / 32) + 1 < (slot / 32){
            return steps;
        }
        // Future Epoch Check
        if target.slot() > slot {
            return steps;
        }

        // Step 2: If do, check target is not in canonical chain
        if !canonical_branch.unwrap().contains(&target_node) { 
            println!("\t Target is not in canonical chain");
            println!("\t [RE-org tiggering Vote Mutation]");
            // Step 3a: If the target is not in canonical chain
            let common_ancestor = chain.find_common_ancestor(&head.clone(), &target_node.clone());
            let target_score = chain.get_branch_score(&target_node.clone(), &common_ancestor.clone());
            let canonical_score = chain.get_branch_score(&head.clone(), &common_ancestor.clone());
            // Check if target score is smaller or same (= competting with canonical)
            assert!(canonical_score >= target_score);
            // Step 4: Get diffence of score between target and head
            let diff = canonical_score - target_score;
            // Can not create attestation more than 8 in a slot
            let mut n_votes = std::cmp::min(diff, 8);
            if n_votes == 0 {
                n_votes = 1;
            }
            println!("n_votes: {:?}", n_votes);
            
            let mut is_block_encountered = false;
            let mut block_delayed = false;
            let mut new_steps = Vec::new();
            println!("[DEBUG] make new step for attestation");
            let next_tick = (slot * 12).into();
            // let next_next_tick = ((slot + 1) * 12).into();
            for step in steps.clone() {
                match step {
                    Step::Block { block } => {
                        // println!("[Step Att] Block: {:?}", block);
                        if block == format!("block_{:?}", target.clone().tree_hash_root()) {
                            is_block_encountered = true;
                        }
                        if block_delayed {
                            if is_block_encountered {
                                new_steps.push(Step::Block { block });
                                for idx in 0..n_votes {
                                    let current_state = chain.blocks.get(&target.canonical_root()).unwrap().post_state.clone();
                                    println!("[DEBUG] target: {:?}, slot: {:?}, current_state_slot: {:?}", target.clone().canonical_root(), slot.clone(), current_state.clone().slot());
                                    let attestation = create_attestation(target.clone(), current_state, slot, idx.try_into().unwrap());
                                    println!("[ATTESTATION GENERATED - A] is attesting block: {:?}", attestation.clone().data.beacon_block_root);
                                    println!("\t[DEBUG] Add block and add attestation");
                                    utils::write_attestation_to_file(attestation.clone());
                                    new_steps.push(Step::Attestation { attestation: format!("attestation_{:?}", attestation.tree_hash_root()) });
                                    is_block_encountered = false;
                                    block_delayed = false;
                                }
                            } else {
                                new_steps.push(Step::Block { block });
                            }
                        } else {
                            new_steps.push(Step::Block { block });
                        }
                    },
                    Step::Attestation { attestation } => {
                        new_steps.push(Step::Attestation { attestation });
                    },
                    Step::Tick { tick } => {
                        // println!("[Step Att] Tick: {:?}", tick);
                        if is_block_encountered{
                            if (tick >= next_tick) {
                                if !block_delayed{
                                    if tick == next_tick {
                                        // println!("[Step Att] Here add attestation");
                                        new_steps.push(Step::Tick { tick });
                                        for idx in 0..n_votes {
                                            let current_state = chain.blocks.get(&target.canonical_root()).unwrap().post_state.clone();
                                            println!("[DEBUG] target: {:?}, slot: {:?}, current_state_slot: {:?}", target.clone().canonical_root(), slot.clone(), current_state.clone().slot());
                                            let attestation = create_attestation(target.clone(), current_state, slot, idx.try_into().unwrap());
                                            println!("[ATTESTATION GENERATED - B] is attesting block: {:?}", attestation.clone().data.beacon_block_root);
                                            println!("[DEBUG] Add tick: {:?}, attestation: {:?}", tick, attestation.tree_hash_root());
                                            utils::write_attestation_to_file(attestation.clone());
                                            new_steps.push(Step::Attestation { attestation: format!("attestation_{:?}", attestation.tree_hash_root()) });
                                            is_block_encountered = false;
                                        }
                                    } else {
                                        new_steps.push(Step::Tick { tick: next_tick });
                                        for idx in 0..n_votes {
                                            let current_state = chain.blocks.get(&target.canonical_root()).unwrap().post_state.clone();
                                            println!("[DEBUG] target: {:?}, slot: {:?}, current_state_slot: {:?}", target.clone().canonical_root(), slot.clone(), current_state.clone().slot());
                                            let attestation = create_attestation(target.clone(), current_state, slot, idx.try_into().unwrap());
                                            println!("[ATTESTATION GENERATED - C] is attesting block: {:?}", attestation.clone().data.beacon_block_root);
                                            utils::write_attestation_to_file(attestation.clone());
                                            new_steps.push(Step::Attestation { attestation: format!("attestation_{:?}", attestation.tree_hash_root()) });
                                            is_block_encountered = false;
                                        }
                                        new_steps.push(Step::Tick { tick });
                                        println!("[DEBUG] Add tick: {:?}, attestation, tick: {:?}", next_tick, tick);
                                    }
                                }
                            } else {
                                new_steps.push(Step::Tick { tick });
                            }
                        } else {
                            if (tick >= next_tick) {
                                block_delayed = true;
                            } 
                            new_steps.push(Step::Tick { tick });
                        }
                    },
                    _ => (),
                };
            }
            // println!("[DEBUG] Attestation added Steps: {:?}", new_steps);
            new_steps
        } else { 
            println!("\t Target is in canonical chain");
            let mut rng = rand::thread_rng();
            // Step 3b: If target is in canonical chain, 
            if choices[dist.sample(&mut rng)] {
                // Add random attestation (66%)
                println!("\t[Random Vote Mutation] Target is in canonical chain. Add random attestation");
                // Step 4a: Add random attestaton. 
                // This affect the fork-chice result of test case. Also it will make some blocks to be meaningless (early rejected).
                // However it is fine, due to it will be meaningful in the future by series of mutation above.
                let att_target: SignedBeaconBlock<MainnetEthSpec> = chain.get_random_block().unwrap().block;
                let next_slot = att_target.slot() + 1;

                let current_state = chain.blocks.get(&att_target.canonical_root()).unwrap().post_state.clone();

                // This may trigger attester slashing (Allowing duplicate voting)
                let attestation = create_attestation(att_target.clone(), current_state, next_slot, rng.gen_range(0..8));
                utils::write_attestation_to_file(attestation.clone());
                // let mut new_steps: Vec<Step<String, String, String, String>> = steps.clone();
                // new_steps.push(Step::Attestation { attestation: format!("attestation_{:?}", attestation.tree_hash_root()) });
                let mut is_block_encountered = false;
                let mut block_delayed = false;
                let mut new_steps = Vec::new();
                // println!("[DEBUG] make new step for attestation");
                let next_tick = (next_slot * 12).into();
                // let next_next_tick = ((next_slot + 1) * 12).into();
                for step in steps.clone() {
                    match step {
                        Step::Block { block } => {
                            // println!("[Step Att] Block: {:?}", block);
                            if block == format!("block_{:?}", att_target.clone().tree_hash_root()) {
                                is_block_encountered = true;
                            }
                            if block_delayed {
                                if is_block_encountered {
                                    new_steps.push(Step::Block { block });
                                    new_steps.push(Step::Attestation { attestation: format!("attestation_{:?}", attestation.tree_hash_root()) });
                                    println!("[ATTESTATION GENERATED - D] is attesting block: {:?}", attestation.clone().data.beacon_block_root);
                                    
                                    is_block_encountered = false;
                                    block_delayed = false;
                                } else {
                                    new_steps.push(Step::Block { block });
                                }
                            } else {
                                new_steps.push(Step::Block { block });
                            }
                        },
                        Step::Attestation { attestation } => {
                            new_steps.push(Step::Attestation { attestation });
                        },
                        Step::Tick { tick } => {
                            if is_block_encountered {
                                if (tick >= next_tick) {
                                    if tick == next_tick {
                                        new_steps.push(Step::Tick { tick });
                                        new_steps.push(Step::Attestation { attestation: format!("attestation_{:?}", attestation.tree_hash_root()) });
                                        println!("[ATTESTATION GENERATED - E] is attesting block: {:?}", attestation.clone().data.beacon_block_root);
                                        is_block_encountered = false;
                                    } else {
                                        new_steps.push(Step::Tick { tick: next_tick });
                                        new_steps.push(Step::Attestation { attestation: format!("attestation_{:?}", attestation.tree_hash_root()) });
                                        println!("[ATTESTATION GENERATED - F] is attesting block: {:?}", attestation.clone().data.beacon_block_root);
                                        is_block_encountered = false;
                                        new_steps.push(Step::Tick { tick });
                                    }
                                } else {
                                    new_steps.push(Step::Tick { tick });
                                }
                            } else {
                                if (tick >= next_tick) {
                                    block_delayed = true;
                                } 
                                new_steps.push(Step::Tick { tick });
                            }
                            
                        },
                        _ => (),
                    };
                }
                // println!("[DEBUG] Attestation added Steps: {:?}", new_steps);
                new_steps
            } else {
                // Do noting (33%)
                println!("\t[Do not Vote Mutation] Just add a block to canonical chain");
                steps.clone()
            }
        }
    } else { 
        // Do not muatate vote (33%)
        println!("\t[Do not Vote Mutation] Just add a block to chain without any attestation");
        steps.clone()
    }
}

pub fn add_random_attestation(
    steps: Vec<Step<String, String, String, String>>, 
) -> Vec<Step<String, String, String, String>> {
    println!("\t[Attestation Mutation] Add random attestation to random block");
    let mut rng = rand::thread_rng();
    let mut blocks_str: Vec::<String> = Vec::new();
    let mut attestations_str: Vec::<String> = Vec::new();
    for step in steps.clone() {
        match step {
            Step::Block { block } => {
                blocks_str.push(format!("{}", block));
            },
            Step::Attestation { attestation } => {
                attestations_str.push(format!("{}", attestation));
            },
           _ => (),
        }
    }
    let sigened_beacon_blocks = utils::get_blocks_from_strings(blocks_str.clone());
    let attestations = utils::get_attestations_from_strings(attestations_str.clone());

    // Step2: get blockchain tree from blocks
    let mut chain = utils::Chain::new();
    for block in sigened_beacon_blocks.clone() {
        let post_state = chain.get_poststate(block.clone());
        chain.insert_block_to_chain(block, post_state);
    }
    chain.set_vote(attestations, sigened_beacon_blocks.clone());
    chain.set_mutatable_pool();

    let att_target: SignedBeaconBlock<MainnetEthSpec> = chain.get_random_block().unwrap().block;
    let next_slot = att_target.slot() + 1;

    let current_state = chain.blocks.get(&att_target.canonical_root()).unwrap().post_state.clone();

    // This may trigger attester slashing (Allowing duplicate voting)
    let attestation = create_attestation(att_target.clone(), current_state, next_slot, rng.gen_range(0..8));
    utils::write_attestation_to_file(attestation.clone());
    let mut checker = false;
    let mut new_steps = Vec::new();
    // println!("[DEBUG] make new step for attestation");
    for step in steps.clone() {
        match step {
            Step::Block { block } => {
                // println!("[Step Att] Block: {:?}", block);
                if block == format!("block_{:?}", att_target.clone().tree_hash_root()) {
                    checker = true;
                }
                new_steps.push(Step::Block { block });
            },
            Step::Tick { tick } => {
                new_steps.push(Step::Tick { tick });
                // println!("[Step Att] Tick: {:?}", tick);
                if checker {
                    // println!("[Step Att] Here add attestation");
                    new_steps.push(Step::Attestation { attestation: format!("attestation_{:?}", attestation.tree_hash_root()) });
                    checker = false;
                }
            },
            _ => (),
        };
    }
    // println!("[DEBUG] Attestation added Steps: {:?}", new_steps);
    new_steps
}

pub fn create_attestation(
    // tester: tester::TesterforMutaiton<MainnetEthSpec>,
    target_block: SignedBeaconBlock<MainnetEthSpec>,
    current_state_: BeaconState<MainnetEthSpec>,
    slot: Slot,
    comm_index: u32 )
-> Attestation<MainnetEthSpec> {
    let forkname = ForkName::Capella;
    let spec = forkname.make_genesis_spec(<MainnetEthSpec>::default_spec());
    let tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    let mut current_state = current_state_.clone();
    // let slot = target.slot() + 1;
    // let state =  tester.harness.get_current_state();
    current_state.build_committee_cache(RelativeEpoch::Current, &spec).unwrap();
    let bc = current_state.get_beacon_committees_at_slot(slot).unwrap();
    let idx = comm_index as usize;
    // println!("[DEBUG] {:?}", bc);
    // println!("[DEBUG] idx: {:?}", idx);

    let mut attestation = tester.harness.produce_unaggregated_attestation_for_block(
        slot,
        bc[0].index,
        target_block.message().tree_hash_root(),
        Cow::Borrowed(&current_state),
        target_block.state_root(),
        ).unwrap();
    attestation.aggregation_bits.set(idx, true).unwrap();
    // Signature is not required in FORKY test
    // attestation.signature = {
    //     let domain = self.spec.get_domain(
    //         attestation.data.target.epoch,
    //         Domain::BeaconAttester,
    //         &fork,
    //         state.genesis_validators_root(),
    //     );

    //     let message = attestation.data.signing_root(domain);

    //     let mut agg_sig = AggregateSignature::infinity();

    //     agg_sig.add_assign(
    //         &self.validator_keypairs[*validator_index].sk.sign(message),
    //     );

    //     agg_sig
    // };
    // println!("[DEBUG] attestation: {:?}", attestation);
    // unimplemented!()
    attestation
}

// TODO: Not implemented yet
pub fn delete_attestation(
    steps: Vec<Step<String, String, String, String>>) 
-> Vec<Step<String, String, String, String>> {
    println!("\t[Delete Attestation Mode]");
    // Get only blocks from steps
    let attestations: Vec<String> = steps.iter()
        .filter_map(|step| {
            match step {
                Step::Attestation { attestation } => Some(format!("{}", attestation)),
                _ => None,
            }
        })
        .collect();
    // Get leaf blocks
    // let sigened_beacon_blocks = utils::get_blocks_from_strings(blocks);
    // if sigened_beacon_blocks.len() == 1 {
    //     // TODO: CALL mutate block
    //     println!("\t\t[Do Nothing] Cannot delete block because there is only one block.");
    //     return steps;
    // }
    // let leaf_blocks = utils::get_leaf_blocks(sigened_beacon_blocks.clone());
    // let target_block = utils::get_random_block(leaf_blocks);
    // println!("\t\t[Target Block] hash: {:?}, file: block_{:?}", target_block.canonical_root() , target_block.tree_hash_root());

    // let mut new_steps: Vec<Step<String, String, String, String>> = Vec::new();
    // for step in steps.clone() {
    //     match step {
    //         Step::Block { ref block } => {
    //             if block != format!("block_{:?}", target_block.tree_hash_root()).as_str() {
    //                 new_steps.push(step);
    //             }
    //         },
    //         Step::Attestation {ref attestation} => {
    //             // Remove following attestation
    //             let path = utils::get_workspace_dir();
    //             let attestation_raw: Attestation<MainnetEthSpec> = 
    //                 targets::utils::ssz_decode_file(&path.join("attestations")
    //                 .join(format!("{}.ssz_snappy", attestation))).unwrap();
    //             let voted_block = attestation_raw.data.beacon_block_root;
    //             if voted_block != target_block.canonical_root() {
    //                 new_steps.push(step);
    //             } 
    //         },
    //         _ => new_steps.push(step),
    //     }
    // }
    // println!("\t\t[Remove Block] hash: {:?}", target_block.canonical_root());

    // new_steps
    steps.clone()
}

