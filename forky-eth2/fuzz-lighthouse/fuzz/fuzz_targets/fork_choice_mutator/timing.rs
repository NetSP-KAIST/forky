
use types::{Hash256, BeaconState, BeaconBlock, SignedBeaconBlock, 
    BeaconBlockCapella, Slot, BeaconBlockBodyCapella, EthSpec, 
    MainnetEthSpec, ChainSpec, ForkName, Attestation,
    test_utils::{SeedableRng, TestRandom, XorShiftRng}};

use tree_hash::TreeHash;
use rand::{Rng, thread_rng};
use std::cmp;

use targets::fork_choice::Step;
use crate::fork_choice_mutator::utils;
use crate::fork_choice_mutator::attestation;
use crate::fork_choice_mutator::tester;
use futures::executor::block_on;

use crate::fork_choice_mutator::utils::BlockNode;

const SEC_PER_SLOT: u64 = 12;

// Delay tick within a slot
pub fn shuffle_tick_within_slot(
    steps: Vec<Step<String, String, String, String>>
)
-> Vec<Step<String, String, String, String>> {
    // Step1: Find target tick in Step Vector
    let mut rng = thread_rng();
    let mut vindex = Vec::new();
    for i in 0..steps.len() {
        match &steps[i] {
            Step::Tick { tick } => vindex.push(i),
            _ => (),
        }
    }

    if vindex.len() == 0 {
        println!("[Timing Shuffle] No tick in steps");
        return steps;
    }

    assert!(!vindex.is_empty());
    let i = rng.gen_range(0..vindex.len());
    let index = vindex[i];

    // Step2: Calculate new tick
    let old_tick = match &steps[index] {
        Step::Tick { tick } => tick,
        _ => panic!("Not a tick"),
    };
    let (next_tick_index, next_tick) = get_next_tick(&steps, index);

    println!("[Tick Shuffle] Tick index: {:?}", index);
    println!("[Tick Shuffle] Old tick: {:?}", old_tick);
    println!("[Tick Shuffle] Next tick: {:?}", next_tick);
    let slot_first_tick = old_tick / SEC_PER_SLOT * SEC_PER_SLOT;
    let slot_last_tick = old_tick / SEC_PER_SLOT * SEC_PER_SLOT + SEC_PER_SLOT - 1;
    // Next tick may be less than slot_last_tick
    let new_tick_bound = std::cmp::min(next_tick, slot_last_tick);

    assert!(slot_first_tick <= *old_tick && *old_tick <= new_tick_bound);
    let new_tick = rng.gen_range(slot_first_tick..=new_tick_bound);
    println!("[Tick Shuffle] New tick: {:?}", new_tick);

    // Step3: Change Tick
    let mut new_steps = steps.clone();
    new_steps[index] = Step::Tick { tick: new_tick };
    
    new_steps
}

// Delay block within a slot
pub fn shuffle_block_tick_within_slot(
    steps: Vec<Step<String, String, String, String>>
)
-> Vec<Step<String, String, String, String>> {
    // Step1: Find target block in Step Vector
    let mut rng = thread_rng();
    let mut vindex = Vec::new();
    for i in 0..steps.len() {
        match &steps[i] {
            Step::Block { block } => vindex.push(i),
            _ => (),
        }
    }

    if vindex.len() == 0 {
        println!("[Timing Shuffle] No block in steps");
        return steps;
    }

    assert!(!vindex.is_empty());
    let i = rng.gen_range(0..vindex.len());
    let index = vindex[i];

    // Step2: Calculate new tick
    let old_tick = steps[0..index].iter()
    .filter_map(|step| {
        match step {
            Step::Tick { tick } => Some(tick),
            _ => None,
        }
    }).last().unwrap();

    let (next_tick_index, next_tick) = get_next_tick(&steps, index);

    println!("[Timing Shuffle] Block index: {:?}", index);
    println!("[Timing Shuffle] Old tick: {:?}", old_tick);
    println!("[Timing Shuffle] Next tick: {:?}", next_tick);
    let slot_first_tick = old_tick / SEC_PER_SLOT * SEC_PER_SLOT;
    let slot_last_tick = old_tick / SEC_PER_SLOT * SEC_PER_SLOT + SEC_PER_SLOT - 1;
    // Next tick may be less than slot_last_tick
    let new_tick_bound = std::cmp::min(next_tick, slot_last_tick);

    assert!(slot_first_tick <= *old_tick && *old_tick <= new_tick_bound);
    let new_tick = rng.gen_range(slot_first_tick..=new_tick_bound);
    println!("[Timing Shuffle] New tick: {:?}", new_tick);

    // Step3: Find a space for new tick
    let mut new_steps = steps.clone();
    let st = new_steps.remove(index);
    assert!(match st { Step::Block{ .. } => true, _ => false });
    let mut inserted = false;

    for i in 0..new_steps.len() {
        match &new_steps[i] {
            Step::Tick { tick } => {
                if tick == &new_tick {
                    new_steps.insert(i + 1, st.clone());
                    inserted = true;
                    break;
                }
                else if tick > &new_tick {
                    new_steps.insert(i, st.clone());
                    new_steps.insert(i, Step::Tick { tick: new_tick });
                    inserted = true;
                    break;
                }
            },
            _ => (),
        }
    }
    if inserted == false {
        new_steps.push(Step::Tick { tick: new_tick });
        new_steps.push(st.clone());
        inserted = true;
    }

    assert!(inserted);
    new_steps
}

fn get_next_tick(
    steps: &Vec<Step<String, String, String, String>>,
    index: usize,
)
-> (usize, u64) {
    let current_tick = match &steps[index] {
        Step::Tick { tick } => tick,
        _ => panic!("Not a tick"),
    };

    let mut next_tick_index = 9999999;
    let mut next_tick = 999999;
    for i in (index+1)..steps.len() {
        match &steps[i] {
            Step::Tick { tick } => {
                next_tick_index = i;
                next_tick = *tick;
                break;
                
            },
            _ => (),
        }
    }
    (next_tick_index, next_tick)
}

// Delay block receive N slots
pub fn delay_block(
    steps: Vec<Step<String, String, String, String>>,
    n_slot: u16,
    target: Hash256,
) 
-> Vec<Step<String, String, String, String>> {
    // Step1: Find target in Step Vector

    // Step2: Delay N slots and add it to new Step Vector
    
    // Step3: Return new Step Vector
    steps
}

pub fn delay_attestations(
    steps: Vec<Step<String, String, String, String>>,
    n_slot: u16,
    target: Hash256,
)
-> Vec<Step<String, String, String, String>> {
    // Step1: Find target in Step Vector

    // Step2: Delay N slots and add it to new Step Vector
    
    // Step3: Return new Step Vector
    steps
}

// Move step to -2, 4 slots 
// if expected slot of specific operation is N, then move it to random range((N-2)*12, (N+4)*12)
pub fn shake_steps(
    steps: Vec<Step<String, String, String, String>>,
    n_block: u16,
    m_slot: u16,
    targets: Vec<Hash256>,
) 
-> Vec<Step<String, String, String, String>> {
    // Step1: Find targets in Step Vector

    // Step2: Delay N slots

    // Step3: Shake them with random order and add it to new Step Vector
    
    // Step4: Return new Step Vector
    steps
}
