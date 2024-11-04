use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use types::{Hash256, BeaconState, BeaconBlock, SignedBeaconBlock, 
    BeaconBlockCapella, Slot, BeaconBlockBodyCapella, EthSpec, 
    MainnetEthSpec, ChainSpec, ForkName, Attestation,
    test_utils::{SeedableRng, TestRandom, XorShiftRng}};

use tree_hash::TreeHash;
use rand::{Rng, thread_rng};

use targets::fork_choice::Step;
use crate::fork_choice_mutator::utils;
use crate::fork_choice_mutator::attestation;
use crate::fork_choice_mutator::block;
use crate::fork_choice_mutator::tester;
use futures::executor::block_on;

use crate::fork_choice_mutator::utils::BlockNode;

pub fn keep_topology(
    steps: Vec<Step<String, String, String, String>>) 
-> Vec<Step<String, String, String, String>> {
    println!("\t[Keep Topology Mode]");
    // TODO: Call Vote Mutator
    steps
}

pub fn depth_first_add(
    steps: Vec<Step<String, String, String, String>>)
-> Vec<Step<String, String, String, String>> {
    println!("\t[Depth-First Mode]");
    // Step1: get only blocks from steps
    // Get only blocks from steps
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

    // Step3: get leaf block from lonest chain
    let leaf_block = &chain.get_highest_leaf_block();
    println!("\t\t[Target Block] hash: {:?}, file: block_{:?}", leaf_block.canonical_root() , leaf_block.tree_hash_root());

    // // Step4.1: get branch that includes the leaf block
    // let branch = chain.get_branch(leaf_block.canonical_root()).unwrap();
    // let mut pre_blocks = Vec::new();
    // let mut pre_blocks_str = Vec::new();
    // for block_node in branch {
    //     let block = block_node.block;
    //     pre_blocks.push(block.clone());
    //     pre_blocks_str.push(format!("block_{:?}", block.tree_hash_root()));
    // }
    // // Step4.2: get step for branch (ticks and blocks only)
    // let mut steps_for_branch:Vec<Step<String, String, String, String>> = Vec::new();
    // for step in steps.clone() {
    //     match step {
    //         Step::Block { block } => {
    //             if pre_blocks_str.contains(&block) {
    //                 steps_for_branch.push(Step::Block { block });
    //             }
    //         },
    //         Step::Tick { tick } => {
    //             steps_for_branch.push(Step::Tick { tick });
    //         },
    //         _ => (),
    //     };
    // }

    // // Step5: exec and get new pre_state for make_block
    // let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    // let new_tester = tester.exec_blocks_and_get_current_tester(steps_for_branch.clone(), pre_blocks.clone()).unwrap();
    // let current_state = new_tester.harness.get_current_state();
    
    // Step6: add new block and tick to steps
    let mut new_steps: Vec<Step<String, String, String, String>> = steps.clone();
    let last_tick = steps.iter()
    .filter_map(|step| {
        match step {
            Step::Tick { tick } => Some(tick),
            _ => None,
        }
    })
    .last()
    .unwrap();
    let next_slot = (last_tick / 12) + 1;
    let current_slot = next_slot - 1;
    let target_slot = std::cmp::max(current_slot, chain.latest_slot.into());
    let current_state = chain.blocks.get(&leaf_block.canonical_root()).unwrap().post_state.clone();
    let new_block = block::add_unattested_block(leaf_block.clone(), current_state, Slot::new(target_slot));
    
    let next_slot_tick: u64 = ((last_tick / 12) + 1) * 12;
    let tick_step = Step::Tick {
        tick: next_slot_tick,
    };
    let new_step = Step::Block {
        block: format!("block_{:?}", new_block.tree_hash_root()),
    };
    new_steps.push(tick_step);

    // println!("[DEBUG] add attestation");
    let mut att_steps = attestation::vote_mutator(
        new_steps.clone(),
        // steps_for_branch.clone(),
        // pre_blocks.clone(),
        leaf_block.clone(),
        chain.blocks.get(&leaf_block.canonical_root()).unwrap().clone(),
        chain.clone(),
        chain.latest_slot
    );

    att_steps.push(new_step);

    // Step7: write new block to file
    utils::write_block_to_file(new_block);
    att_steps
}

pub fn breadth_first_add(
    steps: Vec<Step<String, String, String, String>>)
-> Vec<Step<String, String, String, String>> {
    println!("\t[Breadth-First Mode]");
    // Step1: get only blocks from steps
    // Get only blocks from steps
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

    // Step3: get non-leaf blocks
    let non_leaf_blocks: Vec<&BlockNode> = chain.get_non_leaf_blocks();

    // Step4: choose one target block from non-leaf blocks
    let mut rng = thread_rng();
    let random_index = rng.gen_range(0..non_leaf_blocks.len());
    let target_block = non_leaf_blocks[random_index].clone().block;
    println!("\t\t[Target Block] hash: {:?}, file: block_{:?}", target_block.canonical_root() , target_block.tree_hash_root());

    // // Step4.1: get branch that includes the target block
    // let branch = chain.get_branch(target_block.canonical_root()).unwrap();
    // let mut pre_blocks = Vec::new();
    // let mut pre_blocks_str = Vec::new();
    // for block_node in branch {
    //     let block = block_node.block;
    //     pre_blocks.push(block.clone());
    //     pre_blocks_str.push(format!("block_{:?}", block.tree_hash_root()));
    // }

    // // Step4.2: get step for branch (ticks and blocks only)
    // let mut steps_for_branch:Vec<Step<String, String, String, String>> = Vec::new();
    // for step in steps.clone() {
    //     match step {
    //         Step::Block { block } => {
    //             if pre_blocks_str.contains(&block) {
    //                 steps_for_branch.push(Step::Block { block });
    //             }
    //         },
    //         Step::Tick { tick } => {
    //             steps_for_branch.push(Step::Tick { tick });
    //         },
    //         _ => (),
    //     };
    // }

    // // Step5: exec and get new pre_state for make_block
    // let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    // let new_tester = tester.exec_blocks_and_get_current_tester(steps_for_branch.clone(), pre_blocks.clone()).unwrap();
    // let current_state = new_tester.harness.get_current_state();
    

    // Step6: add block to the target block
    let mut new_steps: Vec<Step<String, String, String, String>> = steps.clone();
    let last_tick = steps.iter()
    .filter_map(|step| {
        match step {
            Step::Tick { tick } => Some(tick),
            _ => None,
        }
    })
    .last()
    .unwrap();
    let next_slot = (last_tick / 12) + 1;
    let current_slot = next_slot - 1;
    let target_slot = std::cmp::max(current_slot, chain.latest_slot.into());
    let current_state = chain.blocks.get(&target_block.canonical_root()).unwrap().post_state.clone();
    let new_block = block::add_unattested_block(target_block.clone(), current_state.clone(), Slot::new(target_slot));

    let next_slot_tick: u64 = next_slot * 12;
    let tick_step = Step::Tick {
        tick: next_slot_tick,
    };
    let new_step = Step::Block {
        block: format!("block_{:?}", new_block.tree_hash_root()),
    };
    new_steps.push(tick_step);

    // println!("[DEBUG] add attestation");
    let mut att_steps = attestation::vote_mutator(
        new_steps.clone(),
        // steps_for_branch.clone(),
        // pre_blocks.clone(),
        target_block.clone(),
        chain.blocks.get(&target_block.canonical_root()).unwrap().clone(),
        chain.clone(),
        chain.latest_slot
    );

    att_steps.push(new_step);

    // Step7: write new block to file
    utils::write_block_to_file(new_block);
    att_steps
}

pub fn balance_first_add(
    steps: Vec<Step<String, String, String, String>>)
-> Vec<Step<String, String, String, String>> {
    println!("\t[Balance-First Mode]");
    // Step1: get only blocks from steps
    // Get only blocks from steps
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
    // for (root, block_node) in chain.clone().blocks {
    //     println!("[DEBUG] block:{:?}, height:{:?}, vote:{:?}\n\t child:{:?}", root, block_node.height, block_node.vote_count, block_node.childs);
    // }

    // Step3: get shortest leaf block
    let leaf_block = &chain.get_shortest_leaf_block();
    println!("\t\t[Target Block] hash: {:?}, file: block_{:?}", leaf_block.canonical_root() , leaf_block.tree_hash_root());

    // // Step4.1: get branch that includes the leaf block
    // let branch = chain.get_branch(leaf_block.canonical_root()).unwrap();
    // let mut pre_blocks = Vec::new();
    // let mut pre_blocks_str = Vec::new();
    // for block_node in branch {
    //     let block = block_node.block;
    //     pre_blocks.push(block.clone());
    //     pre_blocks_str.push(format!("block_{:?}", block.tree_hash_root()));
    // }
    // // Step4.2: get step for branch (ticks and blocks only)
    // let mut steps_for_branch: Vec<Step<String, String, String, String>> = Vec::new();
    // for step in steps.clone() {
    //     match step {
    //         Step::Block { block } => {
    //             if pre_blocks_str.contains(&block) {
    //                 steps_for_branch.push(Step::Block { block });
    //             }
    //         },
    //         Step::Tick { tick } => {
    //             steps_for_branch.push(Step::Tick { tick });
    //         },
    //         _ => (),
    //     };
    // }

    // // Step5: exec and get new pre_state for make_block
    // let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    // let new_tester = tester.exec_blocks_and_get_current_tester(steps_for_branch.clone(), pre_blocks.clone()).unwrap();
    // let current_state = new_tester.harness.get_current_state();
    
    // Step6: add new block to steps
    let mut new_steps: Vec<Step<String, String, String, String>> = steps.clone();
    let last_tick = steps.iter()
    .filter_map(|step| {
        match step {
            Step::Tick { tick } => Some(tick),
            _ => None,
        }
    })
    .last()
    .unwrap();
    let next_slot = (last_tick / 12) + 1;
    let current_slot = next_slot - 1;
    let current_state = chain.blocks.get(&leaf_block.canonical_root()).unwrap().post_state.clone();

    let target_slot = std::cmp::max(current_slot, chain.latest_slot.into());
    let new_block = block::add_unattested_block(leaf_block.clone(), current_state, Slot::new(target_slot));

    let next_slot_tick: u64 = next_slot * 12;
    let tick_step = Step::Tick {
        tick: next_slot_tick,
    };
    let new_step = Step::Block {
        block: format!("block_{:?}", new_block.tree_hash_root()),
    };
    new_steps.push(tick_step);

    // println!("[DEBUG] add attestation");
    let mut att_steps = attestation::vote_mutator(
        new_steps.clone(),
        // steps_for_branch.clone(),
        // pre_blocks.clone(),
        leaf_block.clone(),
        chain.blocks.get(&leaf_block.canonical_root()).unwrap().clone(),
        chain.clone(),
        chain.latest_slot
    );

    att_steps.push(new_step);

    // Step7: write new block to file
    utils::write_block_to_file(new_block);
    att_steps
}