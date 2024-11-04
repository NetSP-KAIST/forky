use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use types::{Hash256, BeaconState, BeaconBlock, SignedBeaconBlock, 
    BeaconBlockCapella, Slot, BeaconBlockBodyCapella, EthSpec, 
    MainnetEthSpec, ChainSpec, ForkName, Attestation,
    test_utils::{SeedableRng, TestRandom, XorShiftRng}};
// use crate::test_utils::TestRandom;
use test_random_derive::TestRandom;
use targets::fork_choice::Step;
use crate::fork_choice_mutator::utils;
use crate::fork_choice_mutator::tester;
use tree_hash::TreeHash;

pub use types::test_utils::generate_deterministic_keypairs;
use eth2_interop_keypairs::{be_private_key, keypair};
use num_bigint::BigUint;
use num_traits::cast::ToPrimitive;
use futures::executor::block_on;

use beacon_chain::test_utils::SyncCommitteeStrategy;

// pub fn mutate_with_arbitrary(raw_data: &[u8]) -> SignedBeaconBlock<MainnetEthSpec> {
//     let mut unstructured = Unstructured::new(raw_data);
//     let new_block: SignedBeaconBlock<MainnetEthSpec> = SignedBeaconBlock::arbitrary(&mut unstructured)
//         .expect("Failed to generate arbitrary block");
//     new_block
// }

pub fn delete_block(
    steps: Vec<Step<String, String, String, String>>) 
-> Vec<Step<String, String, String, String>> {
    println!("\t[Delete Block Mode]");
    // Get only blocks from steps
    let blocks: Vec<String> = steps.iter()
        .filter_map(|step| {
            match step {
                Step::Block { block } => Some(format!("{}", block)),
                _ => None,
            }
        })
        .collect();
    // Get leaf blocks
    let sigened_beacon_blocks = utils::get_blocks_from_strings(blocks);
    if sigened_beacon_blocks.len() <= 2 {
        // TODO: CALL mutate block
        println!("\t\t[Do Nothing] Cannot delete block because there is only one block.");
        return steps;
    }
    let leaf_blocks = utils::get_leaf_blocks(sigened_beacon_blocks.clone());
    if leaf_blocks.len() <= 2 { 
        println!("\t\t[Do Nothing] Cannot delete block because there is no leaf block.");
        return steps;
    }
    let target_block = utils::get_random_block(leaf_blocks);
    println!("\t\t[Target Block] hash: {:?}, file: block_{:?}", target_block.canonical_root() , target_block.tree_hash_root());

    let mut new_steps: Vec<Step<String, String, String, String>> = Vec::new();
    for step in steps.clone() {
        match step {
            Step::Block { ref block } => {
                if block != format!("block_{:?}", target_block.tree_hash_root()).as_str() {
                    new_steps.push(step);
                }
            },
            Step::Attestation {ref attestation} => {
                // Remove following attestation
                let path = utils::get_workspace_dir();
                let attestation_raw: Attestation<MainnetEthSpec> = 
                    targets::utils::ssz_decode_file(&path.join("attestations")
                    .join(format!("{}.ssz_snappy", attestation))).unwrap();
                let voted_block = attestation_raw.data.beacon_block_root;
                if voted_block != target_block.canonical_root() {
                    new_steps.push(step);
                } 
            },
            _ => new_steps.push(step),
        }
    }
    println!("\t\t[Remove Block] hash: {:?}", target_block.canonical_root());

    new_steps
}


pub fn add_unattested_block(
    // tester: tester::TesterforMutaiton<MainnetEthSpec>, 
    parent_block: SignedBeaconBlock<MainnetEthSpec>,
    pre_state: BeaconState<MainnetEthSpec>,
    latest_slot: Slot,)
 -> SignedBeaconBlock<MainnetEthSpec> {
    let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    // Get informations from target block
    let (beacon_block, sign) = parent_block.clone().deconstruct();
    let target_block_header = beacon_block.block_header();

    // Create block on latest slot
    let target_slot = latest_slot + 1;
    let (new_block, post_state) = block_on(tester.harness.make_block(
        pre_state, target_slot,
    ));
    
    // Check if the new block is the child of the target block
    assert_eq!(parent_block.canonical_root(), new_block.message().parent_root());

    println!("\t\t[New Blcok Generated] hash: {:?}, file: block_{:?}", new_block.canonical_root() , new_block.tree_hash_root());
    new_block
}

pub fn add_unattested_block_with_state(
    // tester: tester::TesterforMutaiton<MainnetEthSpec>, 
    parent_block: SignedBeaconBlock<MainnetEthSpec>,
    pre_state: BeaconState<MainnetEthSpec>,
    latest_slot: Slot,)
 -> (SignedBeaconBlock<MainnetEthSpec>, BeaconState<MainnetEthSpec>) {
    let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    // Get informations from target block
    let (beacon_block, sign) = parent_block.clone().deconstruct();
    let target_block_header = beacon_block.block_header();

    // Create block on latest slot
    let target_slot = latest_slot + 1;
    let (new_block, post_state) = block_on(tester.harness.make_block(
        pre_state, target_slot,
    ));
    
    // Check if the new block is the child of the target block
    assert_eq!(parent_block.canonical_root(), new_block.message().parent_root());

    println!("\t\t[New Blcok Generated] hash: {:?}, file: block_{:?}", new_block.canonical_root() , new_block.tree_hash_root());
    (new_block, post_state)
}

pub fn add_attested_block(
    tester: tester::TesterforMutaiton<MainnetEthSpec>, 
    parent_block: SignedBeaconBlock<MainnetEthSpec>,
    pre_state: BeaconState<MainnetEthSpec>,
    latest_slot: Slot,)
 -> SignedBeaconBlock<MainnetEthSpec> {
    // let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    // Get informations from target block
    let (beacon_block, sign) = parent_block.clone().deconstruct();
    let target_block_header = beacon_block.block_header();
    let all_validators = tester.harness.get_all_validators();
    // Create block on latest slot
    let target_slot = latest_slot + 1;
    let (new_hash, new_block, post_state) = block_on(tester.harness.add_attested_block_at_slot_with_sync_custom(
         target_slot, 
         pre_state.clone(), 
         pre_state.tree_hash_root(), 
         &all_validators, 
         SyncCommitteeStrategy::NoValidators, 
    )).unwrap();
    
    // Check if the new block is the child of the target block
    assert_eq!(parent_block.canonical_root(), new_block.message().parent_root());

    println!("\t\t[New Blcok Generated] hash: {:?}, file: block_{:?}", new_block.canonical_root() , new_block.tree_hash_root());
    new_block
}

pub fn add_attested_block_with_state(
    tester: tester::TesterforMutaiton<MainnetEthSpec>, 
    parent_block: SignedBeaconBlock<MainnetEthSpec>,
    pre_state: BeaconState<MainnetEthSpec>,
    latest_slot: Slot,)
 -> (SignedBeaconBlock<MainnetEthSpec>, BeaconState<MainnetEthSpec>, tester::TesterforMutaiton<MainnetEthSpec>) {
    // let mut tester = tester::TesterforMutaiton::<MainnetEthSpec>::setup_new_tester().unwrap();
    // Get informations from target block
    let (beacon_block, sign) = parent_block.clone().deconstruct();
    let target_block_header = beacon_block.block_header();
    let all_validators = tester.harness.get_all_validators();
    // Create block on latest slot
    let target_slot = latest_slot + 1;
    let (new_hash, new_block, post_state) = block_on(tester.harness.add_attested_block_at_slot_with_sync_custom(
         target_slot, 
         pre_state.clone(), 
         pre_state.tree_hash_root(), 
         &all_validators, 
         SyncCommitteeStrategy::NoValidators, 
    )).unwrap();
    
    // Check if the new block is the child of the target block
    assert_eq!(parent_block.canonical_root(), new_block.message().parent_root());

    println!("\t\t[New Blcok Generated] hash: {:?}, file: block_{:?}", new_block.canonical_root() , new_block.tree_hash_root());
    (new_block, post_state, tester)
}



pub fn mutate_to_fully_random(block: &SignedBeaconBlock<MainnetEthSpec>) -> SignedBeaconBlock<MainnetEthSpec> {
    // Generate a new BeaconBlock with random modifications
    // let mut rng = rand::thread_rng();
    let rng = &mut XorShiftRng::from_seed([42; 16]);
    let spec = &ForkName::Capella.make_genesis_spec(MainnetEthSpec::default_spec());
    let inner_block = BeaconBlockCapella {
        slot: Slot::random_for_test(rng),
        proposer_index: u64::random_for_test(rng),
        parent_root: Hash256::random_for_test(rng),
        state_root: Hash256::random_for_test(rng),
        body: BeaconBlockBodyCapella::random_for_test(rng),
    };
    let new_block = BeaconBlock::Capella(inner_block.clone());

    // Generate a new SignedBeaconBlock with the same message but different signature
    let new_signed_block = SignedBeaconBlock::from_block(new_block, block.signature().clone());

    new_signed_block
}

pub fn mutate_with_random(block: &SignedBeaconBlock<MainnetEthSpec>) -> SignedBeaconBlock<MainnetEthSpec> {
    // Generate a new BeaconBlock with random modifications
    // let mut rng = rand::thread_rng();
    let rng = &mut XorShiftRng::from_seed([42; 16]);
    let spec = &ForkName::Capella.make_genesis_spec(MainnetEthSpec::default_spec());
    let inner_block = BeaconBlockCapella {
        slot: block.message().slot().clone(), // Same slot
        proposer_index: u64::random_for_test(rng), // Random proposer index
        parent_root: block.message().parent_root().clone(), // Same parent root
        state_root: Hash256::random_for_test(rng), // Random state root
        body: BeaconBlockBodyCapella::random_for_test(rng), // Random body
    };
    let new_block = BeaconBlock::Capella(inner_block.clone());

    // Generate a new SignedBeaconBlock with the same message but different signature
    let new_signed_block = SignedBeaconBlock::from_block(new_block, block.signature().clone());

    new_signed_block
}

pub async fn mutate_to_valid(
        block: &SignedBeaconBlock<MainnetEthSpec>, 
        tester: tester::TesterforMutaiton<MainnetEthSpec>,) 
    -> SignedBeaconBlock<MainnetEthSpec> {
    
    println!("[DEBUG] INSIDE:TESTER CURRENT SLOT: {:?}", tester.harness.get_current_slot());
    let state = tester.harness.get_current_state();

    let (org_block, org_sign) = block.clone().deconstruct();
    let keys = eth2_interop_keypairs::keypair(
        state.get_beacon_proposer_index(
            org_block.slot(), 
            &tester.harness.chain.spec.clone(),
        )
        .unwrap());

    let pubkey_ = keys.pk;

    println!("[DEBUG] pubkey__{:?}",  pubkey_);
    let blk_new = org_block.sign(
        &keys.sk,
        &state.fork(),
        state.genesis_validators_root(),
        &tester.harness.spec);
    let (new_block, sign_new) = blk_new.clone().deconstruct();
    println!("[DEBUG] sign_old: {:?}", org_sign);
    println!("[DEBUG] sign_new: {:?}", sign_new);

    let validators_ = state.validators();
    // for validator in validators_.clone(){
    //     println!("[Validator PubKey]: {:?}", validator.pubkey);
    // }
    // for n in 1..257 {
    //     let keypair = utils::keypair(n);
    //     println!("[New Validator PubKey]: {:?}", keypair.pk);
    // }

    // println!("Target Block: {:?}", block.clone().tree_hash_root());
    // println!("New Block: {:?}", blk_new.clone().tree_hash_root());
    // // utils::write_block_to_file(blk_new.clone());

    println!("[DEBUG] Before Process: {:?}", tester.harness.get_current_state().canonical_root());
    println!("[DEBUG][PROCESS RESULT:]{:?}", tester.process_block(blk_new.clone(), true));
    println!("[DEBUG] After Process: {:?}", tester.harness.get_current_state().canonical_root());
    // println!("[DEBUG] INSIDE:TESTER PROPOSER INDEX: {:?}", state.get_beacon_proposer_index(tester.harness.get_current_slot(), &ForkName::Capella.make_genesis_spec(MainnetEthSpec::default_spec())).unwrap());
    // println!("[DEBUG] INSIDE:keypair_len: {:?}", tester.harness.validator_keypairs.len());
    // let (new_block, new_state) = tester.harness.make_block(state.clone(), tester.harness.get_current_slot()).await;

    blk_new
}