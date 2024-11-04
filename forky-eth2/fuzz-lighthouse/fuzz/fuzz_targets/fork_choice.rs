#![no_main]
extern crate libfuzzer_sys;
use libfuzzer_sys::{Corpus, fuzz_target};

extern crate targets;
use targets::fuzz_lighthouse_fork_choice as fuzz_target;


extern crate lazy_static;
use lazy_static::lazy_static;

extern crate walkdir;
use walkdir::WalkDir;

extern crate types;
extern crate ssz;
extern crate ssz_derive;

use types::{BeaconState, EthSpec, MainnetEthSpec};

use std::fs::{File};
use std::io;
use std::io::Read;
use std::process;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::str;

extern crate rand;
use crate::rand::distributions::Distribution;
use rand::thread_rng;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rand::distributions::WeightedIndex;
use std::time::Instant;

// Disable/Enable coverage feedback for evaluation
const IS_COVERAGE_FEEDBACK_ENABLED: bool = true;

fuzz_target!(|data: &[u8]| -> Corpus {
    println!("[FORKY] Start fuzzing iteration");
    if !data.is_empty() {
        // Force write to corpus, if test case do not have new reorg-type, removed from corpus
        fork_choice_mutator::utils::write_testcase_to_corpus(data);
    }
    let reorg_feedback = fuzz_target(data);
    if IS_COVERAGE_FEEDBACK_ENABLED {
        if !reorg_feedback {
            // No new reorg type.
            println!("[FORKY] Remove testcase, libfuzzer will decided based on coverage");
            // remove
            fork_choice_mutator::utils::remove_testcase_from_corpus(data);
        }
        // Anyway Accept. The input may be added to the corpus.
        // if libfuzzer find new coverage, libfuzzer will save it to corpus
        println!("[FORKY] End fuzzing iteration");
        return Corpus::Keep;
    } else {
        if !reorg_feedback {
            println!("[FORKY] Remove testcase, not interesting in this case due to we disable coverage feedback");
            fork_choice_mutator::utils::remove_testcase_from_corpus(data);
            println!("[FORKY] End fuzzing iteration");
            return Corpus::Reject;
        }
        println!("[FORKY] End fuzzing iteration");
        return Corpus::Reject;
    }
    // Default
    // May Unreachable
    println!("[FORKY] End fuzzing iteration");
    return Corpus::Keep;
});

mod fork_choice_mutator;
use targets::fork_choice::Step;

libfuzzer_sys::fuzz_mutator!(|data: &mut [u8], size: usize, max_size: usize, seed: u32| {
    let now = Instant::now();
    println!("[DEBUG] mutator old size: {:?}", size);
    let temp = str::from_utf8(&data[..size]).map_err(|e| {
        println!("ERROR: Unable to read test case on mutator: {:?}", e)
    }).unwrap();
    println!("[DEBUG] mutator old data: \n\t{:?}", temp);
    let mut steps: Vec<Step<String, String, String, String>> = targets::utils::yaml_decode_bin(&data[..size]).unwrap();

    let mut rng = StdRng::seed_from_u64(seed as u64);
    let mut rng2 = thread_rng();

    let mut new_steps: Vec<Step<String, String, String, String>> = steps.clone();
    if fork_choice_mutator::utils::check_mut_epoch_enabled() {
        let choices = [0, 1, 2];
        let weights = [80, 5, 15];
        let dist = WeightedIndex::new(&weights).unwrap();
        
        new_steps = match choices[dist.sample(&mut rng2)] {
            // Now branch mutation itself includes attestation mutation
            0 => fork_choice_mutator::mutate_branch(&mut steps, &rng, size, max_size),
            1 => fork_choice_mutator::mutate_epoch(&mut steps, &rng, size, max_size),
            2 => fork_choice_mutator::mutate_timing(&mut steps, &rng, size, max_size),
            // 3 => fork_choice_mutator::mutate_attestation(&mut steps, &rng, size, max_size),
            _ => unreachable!(),
        };
    } else{
        let choices = [0, 1];
        let weights = [80, 20];
        let dist = WeightedIndex::new(&weights).unwrap();
        new_steps = match choices[dist.sample(&mut rng2)] {
            0 => fork_choice_mutator::mutate_branch(&mut steps, &rng, size, max_size),
            1 => fork_choice_mutator::mutate_timing(&mut steps, &rng, size, max_size),
            _ => unreachable!(),
        };
    }
    
    /* FOR TESTING */ 
    // let new_steps = fork_choice_mutator::mutate_branch(&mut steps, &rng, size, max_size);
    // let new_steps = fork_choice_mutator::mutate_epoch(&mut steps, &rng, size, max_size);
    // let new_steps = fork_choice_mutator::mutate_attestation(&mut steps, &rng, size, max_size);
    
    // Sort unsorted ticks - 
    /* 
        deprecated
        Try mutation it self do not create unsorted test case
     */
    // if !fork_choice_mutator::utils::check_step_sorted(new_steps.clone()) {
    //     println!("[DEBUG] Mutator: sort steps");
    //     let new_step = fork_choice_mutator::utils::sort_step(new_steps.clone());
    // } 
    
    // Save generated test case
    let new_yaml = fork_choice_mutator::utils::yaml_encode_with_steps(new_steps);
    println!("[DEBUG] mutator new data: \n\t{:?}", new_yaml);
    fork_choice_mutator::utils::write_testcase_to_file(new_yaml.clone());
    let new_size = new_yaml.len();
    println!("[DEBUG] mutator new size: {:?}", new_size);
    println!("[DEBUG] mutator new data: \n\t{:?}", new_yaml);
    let new_bin = new_yaml.as_bytes();
    data[..new_size].copy_from_slice(&new_bin[..new_size]);

    let elapsed = now.elapsed();
    println!("[FORKY - Mutator] Mutator Elapsed: {:.2?}", elapsed);
    new_size
});

