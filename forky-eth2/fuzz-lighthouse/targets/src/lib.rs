extern crate ssz;
use std::env;

pub mod utils;

use types::{
    BeaconState, SignedBeaconBlock, EthSpec, MainnetEthSpec, ForkName
};
use utils::{testing_spec};

mod block;

extern crate lazy_static;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref VECTORS: Mutex<Vec<utils::ReorgType>> = Mutex::new(Vec::new());
    static ref ITER: Mutex<u32> = Mutex::new(0);
}

fn reorg_feedback(reorg_vectors: Vec<utils::ReorgType>, iter: u32) -> bool {
    if reorg_vectors.is_empty() {
        let counter = VECTORS.lock().unwrap().len() as u32;
        let _ = utils::write_reorg_type_report(iter, counter, reorg_vectors);
        return false;
    }

    let mut new_elements = reorg_vectors.clone().into_iter().filter(|elem_b| {
        let vectors = VECTORS.lock().unwrap();
        !vectors.contains(elem_b)
    }).collect::<Vec<_>>();

    let mut vectors = VECTORS.lock().unwrap();
    let old_len = vectors.len();
    vectors.extend(new_elements.iter().cloned());
    let new_len = vectors.len();

    let _ = utils::write_reorg_type_report(iter, new_len as u32, reorg_vectors);
    println!("[DEBUG] old_len: {:?}, new_len: {:?}", old_len, new_len);
    new_len > old_len
}

#[inline(always)]
pub fn fuzz_lighthouse_block(beaconstate: BeaconState<MainnetEthSpec>, data: &[u8]) {
    let spec = &MainnetEthSpec::default_spec();
    let block = match SignedBeaconBlock::from_ssz_bytes(&data, spec) {
        Ok(block) => block,
        Err(_e) => return,
    };

    let _ = block::state_transition(beaconstate, block, true);
}

pub mod fork_choice;
#[inline(always)]
pub fn fuzz_lighthouse_fork_choice(data: &[u8]) -> bool {
    let forkname = ForkName::Capella;
    let workspace_path = utils::get_workspace_dir();
    println!("workspace_path: {:?}", workspace_path);
    
    let mut reorg_vectors = Vec::new();
    if !data.is_empty() {
        let test_case = 
            fork_choice::ForkChoiceTest::<MainnetEthSpec>::load_testcase_from_yaml(data, 
                forkname, &workspace_path).unwrap();
        let mut iter = ITER.lock().unwrap();
        *iter += 1;
        reorg_vectors = fork_choice::ForkChoiceTest::<MainnetEthSpec>::run(test_case, forkname).unwrap();
        return reorg_feedback(reorg_vectors, *iter);
    } 
    true
    // unsafe{ println!("Reorg Vectors: {:?}", VECTORS ) }
 }