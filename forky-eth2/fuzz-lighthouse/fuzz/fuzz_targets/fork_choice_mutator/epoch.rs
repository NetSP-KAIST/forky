
use types::{Hash256, BeaconState, BeaconBlock, SignedBeaconBlock, 
    BeaconBlockCapella, Slot, BeaconBlockBodyCapella, EthSpec, 
    MainnetEthSpec, ChainSpec, ForkName, Attestation,
    test_utils::{SeedableRng, TestRandom, XorShiftRng}};

use tree_hash::TreeHash;
use crate::rand::{Rng, thread_rng};

use targets::fork_choice::Step;
use crate::fork_choice_mutator::utils;
use crate::fork_choice_mutator::attestation;
use crate::fork_choice_mutator::tester;
use crate::fork_choice_mutator::block;
use futures::executor::block_on;

use crate::fork_choice_mutator::utils::BlockNode;

const SEC_PER_SLOT: u64 = 12;
const SLOT_PER_EPOCH: u64 = 32;

fn fill_left_slots_until_next_epoch(
    steps: Vec<Step<String, String, String, String>>,)
-> Vec<Step<String, String, String, String>> {
    // Step1: Find last slot
    println!("\t[Fill Current Epoch with unattested Blocks & Go to Next Epcoh]");
    let mut new_steps = steps.clone();
    let last_tick = steps.iter()
        .filter_map(|step| {
            match step {
                Step::Tick { tick } => Some(tick),
                _ => None,
            }
        })
        .last()
        .unwrap();
    
    // Step2: Check Epoch of last slot
    let current_slot = last_tick / SEC_PER_SLOT;
    let current_epoch = last_tick / (SEC_PER_SLOT * SLOT_PER_EPOCH);
    let next_epoch = current_epoch + 1;
    let next_epoch_start_tick = next_epoch * (SEC_PER_SLOT * SLOT_PER_EPOCH);
    if (current_slot + 1) * SEC_PER_SLOT == next_epoch_start_tick {
        println!("\t[DEBUG] Already at the end of Epoch");
        // Last tick is already at the end of Epoch
        return new_steps;
    } else if ((current_slot * SEC_PER_SLOT) % SLOT_PER_EPOCH) == 0 {
        // Already at start of Epoch
        println!("\t[DEBUG] Already at start of Epoch");
        return new_steps;
    } else {
        let mut blocks_str: Vec::<String> = Vec::new();
        let mut attestations_str: Vec::<String> = Vec::new();
        for step in new_steps.clone() {
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
        println!("[DEBUG] chain: {:?}", chain.current_justified_checkpoint);
        for node in chain.get_leaf_blocks(){
            println!("\t[DEBUG] leaf: {:?}", node.root);
        }
        println!("[DEBUG] chain slot:{:?}, chain_depth:{:?}", chain.latest_slot, chain.depth);
        for block_node in chain.blocks.values() {
            let hash = block_node.root;
            let parent = block_node.parent_root;
            let vote = block_node.vote_count;
    
            println!("\t[DEBUG] block: {:?}, parent: {:?}, vote: {:?}", hash, parent, vote);
        }
    
        let head = chain.get_canonical_leaf_node().unwrap();
        println!("[DEBUG] result of get_canonical_leaf_node: hash: {:?}, slot:{:?}", head.root, head.block.slot());
        let mut block_now = head.block.clone();
        let mut state_now = head.post_state.clone();
        let mut slot_now = current_slot;

        println!("[DEBUG!!!!!!] slot_now: {:?}", slot_now);
        while (slot_now * SEC_PER_SLOT) < next_epoch_start_tick {
            println!("\t\t[Add Block] current tick: {:?}, next_epoch: {:?}", slot_now * SEC_PER_SLOT, next_epoch_start_tick);
            // Perform other operations within the loop if needed
            let tmp: Slot = slot_now.into();
            println!("\t\t[Add Block] current slot: {:?}", tmp);
            // let block_now = tester_now.find_head().unwrap();
            let (new_block, new_state) = block::add_unattested_block_with_state(block_now.clone(), state_now, tmp);
            
            utils::write_block_to_file(new_block.clone());
            slot_now += 1;
            let new_tick = Step::Tick {
                tick: slot_now * SEC_PER_SLOT,
            };
            let new_block_step = Step::Block {
                block: format!("block_{:?}", new_block.tree_hash_root()),
            };
            new_steps.push(new_tick);
            new_steps.push(new_block_step);
            
            block_now = new_block;
            state_now = new_state;
        }
        
    }
    new_steps
}

pub fn skip_current_epoch(
    steps: Vec<Step<String, String, String, String>>,) 
-> Vec<Step<String, String, String, String>> {
    println!("\t[Skip Current Epoch]");
    let mut new_steps = steps.clone();
    let last_tick = steps.iter()
        .filter_map(|step| {
            match step {
                Step::Tick { tick } => Some(tick),
                _ => None,
            }
        })
        .last()
        .unwrap();

    let current_slot = last_tick / SEC_PER_SLOT;
    // let n_th_in_epoch = current_slot % SLOT_PER_EPOCH;
    let current_epoch = last_tick / (SEC_PER_SLOT * SLOT_PER_EPOCH);
    let next_epoch = current_epoch + 1;
    let next_epoch_start_tick = next_epoch * (SEC_PER_SLOT * SLOT_PER_EPOCH);
    
    if (current_slot + 1) * SEC_PER_SLOT == next_epoch_start_tick {
        // Last tick is already at the end of Epoch
        println!("[DEBUG] Last tick is already at the end of Epoch");
        return new_steps;
    } else if ((current_slot * SEC_PER_SLOT) % SLOT_PER_EPOCH) == 0 {
        println!("[DEBUG] Already at start of Epoch");
        // Already at start of Epoch
        return new_steps;
    } else {
        println!("[DEBUG] Add next_epoch_start_tick: {:?}", next_epoch_start_tick);
        let tick_step = Step::Tick {
            tick: next_epoch_start_tick - SEC_PER_SLOT,
        };
        new_steps.push(tick_step);
        return new_steps
    }
}

// Go to next Epoch
pub fn next_epoch(
    steps: Vec<Step<String, String, String, String>>,) 
-> Vec<Step<String, String, String, String>> {
    let mut rng = rand::thread_rng();
    match rng.gen_range(0..2) {
        0 => fill_left_slots_until_next_epoch(steps),
        1 => skip_current_epoch(steps),
        _ => unreachable!(),
    }
    // Last tick is at the end of Epoch
}

pub fn add_full_epoch_with_skip_left_slots(
    steps: Vec<Step<String, String, String, String>>,
)
-> Vec<Step<String, String, String, String>> {
    println!("\t[Fill Next Epoch with attested Blocks Mode]");
    let mut new_steps = next_epoch(steps.clone());
    // Last tick is right before of new Epoch
    let mut largest_tick = 0;
    for step in new_steps.clone() {
        match step {
            Step::Tick { tick } => {
                if tick > largest_tick {
                    largest_tick = tick;
                }
            }
            _ => {}
        }
    }
    let last_tick = largest_tick;
    println!("[DEBUG] last_tick: {:?}", last_tick);

    

    let current_slot = last_tick / SEC_PER_SLOT;
    let current_epoch = last_tick / (SEC_PER_SLOT * SLOT_PER_EPOCH);
    println!("[DEBUG] current_slot: {:?}, current_epoch: {:?}", current_slot, current_epoch);

    // This is start of new Epoch, now we are going to fill this
    let mut next_epoch = 0;
    if ((last_tick / SEC_PER_SLOT) % SLOT_PER_EPOCH) == 31 {
        println!("\t[DEBUG] Already at the end of Epoch");
        // Last tick is already at the end of past Epoch
        // This is start of new Epoch, now we are going to fill this
        next_epoch = current_epoch + 2;
    } else {
        // At start of Epoch
        next_epoch = current_epoch + 1;
    } 
    assert_ne!(next_epoch, 0);
    // End of working Epoch and start of new Epoch
    let next_epoch_start_tick = next_epoch * (SEC_PER_SLOT * SLOT_PER_EPOCH);
    
    let mut blocks_str: Vec::<String> = Vec::new();
    let mut attestations_str: Vec::<String> = Vec::new();
    for step in new_steps.clone() {
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
    println!("[DEBUG] chain slot:{:?}, chain_depth:{:?}", chain.latest_slot, chain.depth);
    for block_node in chain.blocks.values() {
        let hash = block_node.root;
        let parent = block_node.parent_root;
        let vote = block_node.vote_count;

        println!("\t[DEBUG] block: {:?}, parent: {:?}, vote: {:?}", hash, parent, vote);
    }


    let head = chain.get_canonical_leaf_node().unwrap();
    println!("[DEBUG] result of get_canonical_leaf_node: hash: {:?}, slot:{:?}", head.root, head.block.slot());
    let mut block_now = head.block.clone();
    let mut state_now = head.post_state.clone();
    let mut slot_now = current_slot;

    let branch = chain.get_branch(head.root).unwrap();
    let mut pre_blocks = Vec::new();
    let mut pre_blocks_str = Vec::new();
    for block_node in branch {
        let block = block_node.block;
        pre_blocks.push(block.clone());
        pre_blocks_str.push(format!("block_{:?}", block.tree_hash_root()));
    }
    // Step4.2: get step for branch (ticks and blocks only)
    let mut steps_for_branch:Vec<Step<String, String, String, String>> = Vec::new();
    for step in new_steps.clone() {
        match step {
            Step::Block { block } => {
                if pre_blocks_str.contains(&block) {
                    steps_for_branch.push(Step::Block { block });
                }
            },
            Step::Tick { tick } => {
                steps_for_branch.push(Step::Tick { tick });
            },
            _ => (),
        };
    }
    println!("[DEBUG] steps_for_branch: {:?}", steps_for_branch);

    // Step5: exec and get new pre_state for make_block
    let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    let mut tester_now = tester.exec_blocks_and_get_current_tester(steps_for_branch.clone(), pre_blocks.clone()).unwrap();
    // let current_state = new_tester.harness.get_current_state();

    while (slot_now * SEC_PER_SLOT) < next_epoch_start_tick {
        println!("\t\t[Add Block] current tick: {:?}, next_epoch: {:?}", slot_now * SEC_PER_SLOT, next_epoch_start_tick);
        // Perform other operations within the loop if needed
        let tmp: Slot = slot_now.into();
        println!("\t\t[Add Block] current slot: {:?}", tmp);
        // let block_now = tester_now.find_head().unwrap();
        let (new_block, new_state, new_tester) = block::add_attested_block_with_state(tester_now, block_now.clone(), state_now, tmp);
        utils::write_block_to_file(new_block.clone());
        slot_now += 1;
        let new_tick = Step::Tick {
            tick: slot_now * SEC_PER_SLOT,
        };
        let new_block_step = Step::Block {
            block: format!("block_{:?}", new_block.tree_hash_root()),
        };
        new_steps.push(new_tick);
        new_steps.push(new_block_step);
        
        
        block_now = new_block;
        state_now = new_state;
        tester_now = new_tester;
    }

    new_steps
}

pub fn add_justifiable_epoch_with_skip_left_slots(
    steps: Vec<Step<String, String, String, String>>,
)
-> Vec<Step<String, String, String, String>> {
    println!("\t[Fill Next Epoch with justifiable attested Blocks Mode]");
    let mut new_steps = next_epoch(steps.clone());

    // Last tick is right before of new Epoch
    let mut largest_tick = 0;
    for step in new_steps.clone() {
        match step {
            Step::Tick { tick } => {
                if tick > largest_tick {
                    largest_tick = tick;
                }
            }
            _ => {}
        }
    }
    let last_tick = largest_tick;
    println!("[DEBUG] last_tick: {:?}", last_tick);
  
    let current_slot = last_tick / SEC_PER_SLOT;
    let current_epoch = last_tick / (SEC_PER_SLOT * SLOT_PER_EPOCH);

    // This is start of new Epoch, now we are going to fill this
    let mut next_epoch = 0;
    if ((last_tick / SEC_PER_SLOT) % SLOT_PER_EPOCH) == 31 {
        println!("\t[DEBUG] Already at the end of Epoch");
        // Last tick is already at the end of past Epoch
        // This is start of new Epoch, now we are going to fill this
        next_epoch = current_epoch + 2;
    } else {
        // At start of Epoch
        next_epoch = current_epoch + 1;
    } 
    assert_ne!(next_epoch, 0);
    // End of working Epoch and start of new Epoch
    let next_epoch_start_tick = next_epoch * (SEC_PER_SLOT * SLOT_PER_EPOCH);
    
    let mut blocks_str: Vec::<String> = Vec::new();
    let mut attestations_str: Vec::<String> = Vec::new();
    for step in new_steps.clone() {
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
    println!("[DEBUG] chain slot:{:?}, chain_depth:{:?}", chain.latest_slot, chain.depth);
    for block_node in chain.blocks.values() {
        let hash = block_node.root;
        let parent = block_node.parent_root;
        let vote = block_node.vote_count;

        println!("\t[DEBUG] block: {:?}, parent: {:?}, vote: {:?}", hash, parent, vote);
    }

    let head = chain.get_canonical_leaf_node().unwrap();
    println!("[DEBUG] result of get_canonical_leaf_node: hash: {:?}, slot:{:?}", head.root, head.block.slot());
    let mut block_now = head.block.clone();
    let mut state_now = head.post_state.clone();
    let mut slot_now = current_slot;

    let branch = chain.get_branch(head.root).unwrap();
    let mut pre_blocks = Vec::new();
    let mut pre_blocks_str = Vec::new();
    for block_node in branch {
        let block = block_node.block;
        pre_blocks.push(block.clone());
        pre_blocks_str.push(format!("block_{:?}", block.tree_hash_root()));
    }
    // Step4.2: get step for branch (ticks and blocks only)
    let mut steps_for_branch:Vec<Step<String, String, String, String>> = Vec::new();
    for step in new_steps.clone() {
        match step {
            Step::Block { block } => {
                if pre_blocks_str.contains(&block) {
                    steps_for_branch.push(Step::Block { block });
                }
            },
            Step::Tick { tick } => {
                steps_for_branch.push(Step::Tick { tick });
            },
            _ => (),
        };
    }
    println!("[DEBUG] steps_for_branch: {:?}", steps_for_branch);
    // Step5: exec and get new pre_state for make_block
    let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    let mut tester_now = tester.exec_blocks_and_get_current_tester(steps_for_branch.clone(), pre_blocks.clone()).unwrap();
    // let current_state = new_tester.harness.get_current_state();

    // Generage 22 blocks and skip left slots
    let mut count = 0;
    let mut rng_n_block = thread_rng();
    let nblock = rng_n_block.gen_range(22..31);
    while count < nblock {
        println!("\t\t[Add Block] current tick: {:?}, next_epoch: {:?}", slot_now * SEC_PER_SLOT, next_epoch_start_tick);
        // Perform other operations within the loop if needed
        let tmp: Slot = slot_now.into();
        println!("\t\t[Add Block] current slot: {:?}", tmp);
        let (new_block, new_state, new_tester) = block::add_attested_block_with_state(tester_now, block_now.clone(), state_now, tmp);
        utils::write_block_to_file(new_block.clone());
        slot_now += 1;
        let new_tick = Step::Tick {
            tick: slot_now * SEC_PER_SLOT,
        };
        let new_block_step = Step::Block {
            block: format!("block_{:?}", new_block.tree_hash_root()),
        };
        new_steps.push(new_tick);
        new_steps.push(new_block_step);
        
        block_now = new_block;
        state_now = new_state;
        tester_now = new_tester;
        count += 1;
    }
    println!("[DEBUG] next_epoch_start_tick: {:?}", next_epoch_start_tick);
    let new_tick = Step::Tick {
        tick: next_epoch_start_tick,
    };
    new_steps.push(new_tick);

    new_steps
}

pub fn add_unjustifiable_epoch_with_skip_left_slots(
    steps: Vec<Step<String, String, String, String>>,
)
-> Vec<Step<String, String, String, String>> {
    println!("\t[Fill Next Epoch with unjustifiable Blocks Mode]");
    let mut new_steps = next_epoch(steps.clone());
    println!("\t[Now in Next Epoch: Start fill Epoch with unjustifiable Blocks]");
    let last_tick = new_steps.iter()
        .filter_map(|step| {
            match step {
                Step::Tick { tick } => Some(tick),
                _ => None,
            }
        })
        .last()
        .unwrap();
    println!("[DEBUG!!!] last_tick: {:?}", last_tick);

    let current_slot = last_tick / SEC_PER_SLOT;
    let current_epoch = last_tick / (SEC_PER_SLOT * SLOT_PER_EPOCH);
    
    // This is start of new Epoch, now we are going to fill this
    let mut next_epoch = 0;
    if ((last_tick / SEC_PER_SLOT) % SLOT_PER_EPOCH) == 31 {
        println!("\t[DEBUG] Already at the end of Epoch");
        // Last tick is already at the end of past Epoch
        // This is start of new Epoch, now we are going to fill this
        next_epoch = current_epoch + 2;
    } else {
        // At start of Epoch
        next_epoch = current_epoch + 1;
    } 
    assert_ne!(next_epoch, 0);
    // End of working Epoch and start of new Epoch
    let next_epoch_start_tick = next_epoch * (SEC_PER_SLOT * SLOT_PER_EPOCH);
    
    let mut blocks_str: Vec::<String> = Vec::new();
    let mut attestations_str: Vec::<String> = Vec::new();
    for step in new_steps.clone() {
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

    println!("[DEBUG] chain slot:{:?}, chain_depth:{:?}", chain.latest_slot, chain.depth);
    for block_node in chain.blocks.values() {
        let hash = block_node.root;
        let parent = block_node.parent_root;
        let vote = block_node.vote_count;

        println!("\t[DEBUG] block: {:?}, parent: {:?}, vote: {:?}", hash, parent, vote);
    }


    let head = chain.get_canonical_leaf_node().unwrap();
    println!("[DEBUG] result of get_canonical_leaf_node: hash: {:?}, slot:{:?}", head.root, head.block.slot());
    let mut block_now = head.block.clone();
    let mut state_now = head.post_state.clone();
    let mut slot_now = current_slot ;

    let branch = chain.get_branch(head.root).unwrap();
    let mut pre_blocks = Vec::new();
    let mut pre_blocks_str = Vec::new();
    for block_node in branch {
        let block = block_node.block;
        pre_blocks.push(block.clone());
        pre_blocks_str.push(format!("block_{:?}", block.tree_hash_root()));
    }
    // Step4.2: get step for branch (ticks and blocks only)
    let mut steps_for_branch:Vec<Step<String, String, String, String>> = Vec::new();
    for step in new_steps.clone() {
        match step {
            Step::Block { block } => {
                if pre_blocks_str.contains(&block) {
                    steps_for_branch.push(Step::Block { block });
                }
            },
            Step::Tick { tick } => {
                steps_for_branch.push(Step::Tick { tick });
            },
            _ => (),
        };
    }
    println!("[DEBUG] steps_for_branch: {:?}", steps_for_branch);
    // Step5: exec and get new pre_state for make_block
    let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    let mut tester_now = tester.exec_blocks_and_get_current_tester(steps_for_branch.clone(), pre_blocks.clone()).unwrap();
    // let current_state = new_tester.harness.get_current_state();

    // Generage 20 blocks and skip left slots
    let mut count = 0;
    while (count <= 20) {
        let mut rng = rand::thread_rng();
        // match rng.gen_range(0..2) {
        //     0 => {
        println!("\t\t[Add Block] current tick: {:?}, next_epoch: {:?}", slot_now * SEC_PER_SLOT, next_epoch_start_tick);
        // Perform other operations within the loop if needed
        
        
        let tmp: Slot = slot_now.into();
        println!("\t\t[Add Block] current slot: {:?}", tmp);
        let (new_block, new_state, new_tester) = block::add_attested_block_with_state(tester_now, block_now.clone(), state_now, tmp);
        utils::write_block_to_file(new_block.clone());
        
        slot_now += 1;
        let new_tick = Step::Tick {
            tick: slot_now * SEC_PER_SLOT,
        };
        let new_block_step = Step::Block {
            block: format!("block_{:?}", new_block.tree_hash_root()),
        };
        
        count += 1;
        println!("\t\t[DEBUG] add unjustifiable: Counter: {:?}", count);
        new_steps.push(new_tick);
        new_steps.push(new_block_step);
        
        block_now = new_block;
        state_now = new_state;
        tester_now = new_tester;
            // },
            // 1 => {
            //     slot_now += 1;
            // },
            // _ => unreachable!(),
        // }
    }
    println!("[DEBUG] next_epoch_start_tick: {:?}", next_epoch_start_tick);
    let new_tick = Step::Tick {
        tick: next_epoch_start_tick - SEC_PER_SLOT,
    };
    new_steps.push(new_tick);

    new_steps
}

// Add 2 Epoch to finalize current Epoch
// This mutation make test cases too big and slow down entire process, 
// Therefore, this mutation opersation should invoke very low probability
pub fn finalize_current_epoch(
    steps: Vec<Step<String, String, String, String>>,
)
-> Vec<Step<String, String, String, String>> {
    println!("\t[Finalize Current Epoch Mode]");
    // Call add_justifiable_epoch_with_skip_left_slots 2 times
    let mut new_steps = next_epoch(steps.clone());
    new_steps = add_justifiable_epoch_with_skip_left_slots(new_steps.clone());
    new_steps = add_justifiable_epoch_with_skip_left_slots(new_steps.clone());

    new_steps
}
