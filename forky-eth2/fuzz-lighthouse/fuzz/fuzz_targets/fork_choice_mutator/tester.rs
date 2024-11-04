extern crate rand;
use crate::fork_choice_mutator::utils;
use rand::{rngs::StdRng, Rng, SeedableRng};
use tree_hash::TreeHash;
use targets::fork_choice::Step;
use targets::fork_choice::PowBlock;
use targets::utils::Error;

use std::path::{Path};
use std::fmt::Debug;
use ::fork_choice::PayloadVerificationStatus;

use types::{
    Attestation, AttesterSlashing, BeaconBlock, BeaconState, Checkpoint, EthSpec, MainnetEthSpec, 
    ExecutionBlockHash, ForkName, Hash256, IndexedAttestation, SignedBeaconBlock, Slot, Uint256,
    ChainSpec, Graffiti, BeaconBlockBodyCapella, BeaconBlockCapella,
};
use beacon_chain::slot_clock::SlotClock;
use beacon_chain::{
    attestation_verification::{
        obtain_indexed_attestation_and_committees_per_slot, VerifiedAttestation,
    },
    test_utils::{BeaconChainHarness, EphemeralHarnessType},
    BeaconChainTypes, CachedHead, CountUnrealized, NotifyExecutionLayer, ProduceBlockVerification,
};

use execution_layer::{json_structures::JsonPayloadStatusV1Status, PayloadStatusV1};
use serde::Deserialize;
use ssz_derive::Decode;
use ssz::{Encode, ssz_encode};
use snap::write::FrameEncoder;
use snap::raw::Encoder;
use std::fs::File;
use std::io::{Write};
use state_processing::state_advance::complete_state_advance;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

pub use types::test_utils::generate_deterministic_keypairs;
use eth2_interop_keypairs::{be_private_key, keypair};
use futures::executor::block_on;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutatorForkChoiceTest<E: EthSpec> {
    pub steps_str: Vec<Step<String, String, String, String>>,
    pub anchor_state: BeaconState<E>,
    pub anchor_block: BeaconBlock<E>,
    #[allow(clippy::type_complexity)]
    pub steps: Vec<Step<SignedBeaconBlock<E>, Attestation<E>, AttesterSlashing<E>, PowBlock>>,
}

impl<E: EthSpec> MutatorForkChoiceTest<E> {
    pub fn set_testcase(steps_str: Vec<Step<String, String, String, String>>)
    -> Result<Self, Error> {
        let path = utils::get_workspace_dir();
        let forkname = ForkName::Capella;
        let spec = forkname.make_genesis_spec(<MainnetEthSpec>::default_spec());
        // let anchor_state = targets::utils::ssz_decode_state(&path.join("anchor_state").join("state_0xf7b5bc3b916e012735430ac8610a7b9941cea06f68df5950d5afd8f7e45560c6.ssz_snappy"), &spec)?;
        let anchor_state = targets::utils::ssz_decode_state(&path.join("anchor_state").join("anchor_state.ssz_snappy"), &spec)?;
        
        // let state = anchor_state.clone();
        // let path_ = path.join("anchor_state")
        //     .join(format!("state_{:?}.ssz_snappy", state.tree_hash_root()));
        // let ssz = state.as_ssz_bytes();
        // // Open the output file
        // let mut file = File::create(path_).unwrap();
        // // Compress the SSZ bytes
        // let mut snappy = {
        //     let mut encoder = FrameEncoder::new(Vec::new());
        //     encoder.write_all(&ssz).unwrap();
        //     encoder.into_inner().unwrap()
        // };
        // // println!("\n");
        // // for b in snappy.clone() {
        // //     print!("{:02x} ", b);
        // // }

        // // Remove the snappy header
        // let mut header = vec![0xC5, 0x9E, 0xA9, 0x01];
        // header.extend_from_slice(&snappy[21..]);
        // file.write_all(&header).unwrap();
        // // file.write_all(&snappy[18..]).unwrap();
        // let try_parse: BeaconState<MainnetEthSpec> = targets::utils::ssz_decode_state(&path.join("anchor_state").join("state_0xf7b5bc3b916e012735430ac8610a7b9941cea06f68df5950d5afd8f7e45560c6.ssz_snappy"), &spec)?;
        // println!("[DEBUG] ===== Parsing Success =====");
        
        let anchor_block = targets::utils::ssz_decode_file_with(&path.join("anchor_block").join("anchor_block.ssz_snappy"), 
            |bytes| {
                BeaconBlock::from_ssz_bytes(bytes, &spec)
            })?;

        let steps: Vec<Step<SignedBeaconBlock<E>, Attestation<E>, AttesterSlashing<E>, PowBlock>> =
        steps_str.clone()
            .into_iter()
            .map(|step| match step {
                Step::Tick { tick } => Ok(Step::Tick { tick }),
                Step::Block { block } => {
                    targets::utils::ssz_decode_file_with(
                        &path.join("blocks")
                        .join(format!("{}.ssz_snappy", block)), 
                        |bytes| {
                            SignedBeaconBlock::from_ssz_bytes(bytes, &spec)
                        })
                        .map(|block| Step::Block { block })
                }
                Step::InvalidBlockCandidate { block, valid } => {
                    targets::utils::ssz_decode_file_with(
                        &path.join("blocks")
                        .join(format!("{}.ssz_snappy", block)), 
                        |bytes| {
                            SignedBeaconBlock::from_ssz_bytes(bytes, &spec)
                        })
                        .map(|block| Step::InvalidBlockCandidate { block, valid })
                }
                Step::Attestation { attestation } => {
                    targets::utils::ssz_decode_file(&path.join("attestations").join(format!("{}.ssz_snappy", attestation)))
                        .map(|attestation| Step::Attestation { attestation })
                }
                Step::AttesterSlashing { attester_slashing } => {
                    targets::utils::ssz_decode_file(&path.join("att_slash").join(format!("{}.ssz_snappy", attester_slashing)))
                        .map(|attester_slashing| Step::AttesterSlashing { attester_slashing })
                }
                Step::PowBlock { pow_block } => {
                    targets::utils::ssz_decode_file(&path.join("pow_blk").join(format!("{}.ssz_snappy", pow_block)))
                        .map(|pow_block| Step::PowBlock { pow_block })
                }
                Step::OnPayloadInfo {
                    block_hash,
                    payload_status,
                } => Ok(Step::OnPayloadInfo {
                    block_hash,
                    payload_status,
                }),
                Step::Checks { checks } => Ok(Step::Checks { checks }),
                _ => unreachable!(),
            })
            .collect::<Result<_, _>>()?;

        Ok(Self {
            steps_str,
            anchor_state,
            anchor_block,
            steps,
        })
    }

    pub fn run_and_get_current_tester(
        testcase: MutatorForkChoiceTest<E>) 
    -> TesterforMutaiton<E> {
        let fork_name = ForkName::Capella;
        let tester = TesterforMutaiton::new(&testcase, fork_name.make_genesis_spec(E::default_spec())).unwrap();

        for step in &testcase.steps {
            match step {
                Step::Tick { tick } => tester.set_tick(*tick),
                Step::Block { block } => {
                    println!("[DEBUG] Step = Block tree hash: {:?}", block.tree_hash_root());
                    println!("[DEBUG] Step = Block canonical hash: {:?}", block.canonical_root());
                    tester.process_block(block.clone(), true).unwrap();
                },
                Step::InvalidBlockCandidate { block, valid } => {
                    // println!("[DEBUG] Step = Block tree hash: {:?}", block.tree_hash_root());
                    // println!("[DEBUG] Step = Block canonical hash: {:?}", block.canonical_root());
                    tester.process_block(block.clone(), *valid).unwrap();
                },
                Step::Attestation { attestation } => {
                    // println!("[DEBUG] Step = Attestation: {:?}", attestation.tree_hash_root());
                    // println!("[DEBUG] Attestation is attesting block: {:?}", attestation.data.beacon_block_root);
                    // println!("[DEBUG] Attestation aggregation bit: {:?}", attestation.aggregation_bits);
                    tester.process_attestation(attestation).unwrap();
                },
                Step::AttesterSlashing { attester_slashing } => {
                    tester.process_attester_slashing(attester_slashing)
                },
                Step::PowBlock { pow_block } => tester.process_pow_block(pow_block),
                Step::OnPayloadInfo {
                    block_hash,
                    payload_status,
                } => {
                    let el = tester.harness.mock_execution_layer.as_ref().unwrap();
                    el.server
                        .set_payload_statuses(*block_hash, payload_status.clone().into());
                },
                _ => (),
            }
        };

        tester
    }

    pub fn run_and_get_canonical_head(
        testcase: MutatorForkChoiceTest<E>) 
    -> Hash256 {
        let fork_name = ForkName::Capella;
        let tester = TesterforMutaiton::new(&testcase, fork_name.make_genesis_spec(E::default_spec())).unwrap();

        for step in &testcase.steps {
            match step {
                Step::Tick { tick } => tester.set_tick(*tick),
                Step::Block { block } => {
                    // println!("[DEBUG] Step = Block tree hash: {:?}", block.tree_hash_root());
                    // println!("[DEBUG] Step = Block canonical hash: {:?}", block.canonical_root());
                    tester.process_block(block.clone(), true).unwrap();
                },
                Step::InvalidBlockCandidate { block, valid } => {
                    // println!("[DEBUG] Step = Block tree hash: {:?}", block.tree_hash_root());
                    // println!("[DEBUG] Step = Block canonical hash: {:?}", block.canonical_root());
                    tester.process_block(block.clone(), *valid).unwrap();
                },
                Step::Attestation { attestation } => {
                    // println!("[DEBUG] Step = Attestation: {:?}", attestation.tree_hash_root());
                    // println!("[DEBUG] Attestation is attesting block: {:?}", attestation.data.beacon_block_root);
                    // println!("[DEBUG] Attestation aggregation bit: {:?}", attestation.aggregation_bits);
                    tester.process_attestation(attestation).unwrap();
                },
                Step::AttesterSlashing { attester_slashing } => {
                    tester.process_attester_slashing(attester_slashing)
                },
                Step::PowBlock { pow_block } => tester.process_pow_block(pow_block),
                Step::OnPayloadInfo {
                    block_hash,
                    payload_status,
                } => {
                    let el = tester.harness.mock_execution_layer.as_ref().unwrap();
                    el.server
                        .set_payload_statuses(*block_hash, payload_status.clone().into());
                },
                _ => (),
            }
        };

        let head = tester.find_head().unwrap();
        head.head_block_root()
    }
}

/// A testing rig used to execute a test case.
pub struct TesterforMutaiton<E: EthSpec> {
    pub harness: BeaconChainHarness<EphemeralHarnessType<E>>,
    spec: ChainSpec,
}

impl<E: EthSpec> TesterforMutaiton<E> {
    pub fn setup_new_tester() 
    -> Result<Self, Error> {
        let path = utils::get_workspace_dir();
        let forkname = ForkName::Capella;
        let spec = forkname.make_genesis_spec(<E>::default_spec());
        // let anchor_state = targets::utils::ssz_decode_state(&path.join("anchor_state").join("state_0xf7b5bc3b916e012735430ac8610a7b9941cea06f68df5950d5afd8f7e45560c6.ssz_snappy"), &spec)?;
        let anchor_state = targets::utils::ssz_decode_state(&path.join("anchor_state").join("anchor_state.ssz_snappy"), &spec)?;
        let anchor_block = targets::utils::ssz_decode_file_with(&path.join("anchor_block").join("anchor_block.ssz_snappy"), 
            |bytes| {
                BeaconBlock::<E>::from_ssz_bytes(bytes, &spec)
            })?;
        
        let genesis_time = anchor_state.genesis_time();

        if anchor_state.slot() != spec.genesis_slot {
            // I would hope that future fork-choice tests would start from a non-genesis anchors,
            // however at the time of writing, none do. I think it would be quite easy to do
            // non-genesis anchors via a weak-subjectivity/checkpoint start.
            //
            // Whilst those tests don't exist, we'll avoid adding checkpoint start complexity to the
            // `BeaconChainHarness` and create a hard failure so we can deal with it then.
            return Err(Error::FailedToParseTest(
                "anchor state is not a genesis state".into(),
            ));
        }

        // Set harness
        let harness = BeaconChainHarness::builder(E::default())
            .spec(spec.clone())
            .keypairs(types::test_utils::load_keypairs_from_yaml(
                utils::get_workspace_dir().join("anchor_state").join("keys.yaml")
            ).unwrap())
            .genesis_state_ephemeral_store(anchor_state.clone())
            .mock_execution_layer()
            .recalculate_fork_times_with_genesis(0)
            .mock_execution_layer_all_payloads_valid()
            .build();
        harness
            .mock_execution_layer
            .as_ref()
            .unwrap()
            .server
            .drop_all_blocks();

        assert_eq!(
            harness.chain.slot_clock.genesis_duration().as_secs(),
            genesis_time
        );
        
        Ok(Self {
            harness,
            spec,
        })
    }

    pub fn exec_blocks_and_get_current_tester(
        self,
        steps: Vec<Step<String, String, String, String>>,
        blocks: Vec<SignedBeaconBlock<E>>,) 
    -> Result<Self, Error> {
        let path = utils::get_workspace_dir();
        let forkname = ForkName::Capella;
        let spec = forkname.make_genesis_spec(<E>::default_spec());

        for step in steps {
            match step {
                Step::Tick { tick } => self.set_tick(tick),
                Step::Block { block } => {
                    let beacon_block = targets::utils::ssz_decode_file_with(&path.join("blocks").join(format!("{}.ssz_snappy", block)), |bytes| {
                        SignedBeaconBlock::from_ssz_bytes(bytes, &spec)
                    }).unwrap();
                    // println!("[DEBUG] [RPOCESSING] exec_blocks_and_get_current_tester: {:?}", block);
                    self.process_block(beacon_block, true).unwrap();
                },
                Step::InvalidBlockCandidate { block, valid } => {
                    let beaconb_block = targets::utils::ssz_decode_file_with(&path.join("blocks").join(format!("{}.ssz_snappy", block)), |bytes| {
                        SignedBeaconBlock::from_ssz_bytes(bytes, &spec)
                    }).unwrap();
                    // println!("[DEBUG] [RPOCESSING] exec_blocks_and_get_current_tester: {:?}", block);
                    self.process_block(beaconb_block, valid).unwrap();
                },
                _ => (),
            }
        }

        // check block process is done and head is correct
        let head = self.find_head().unwrap().head_block_root();
        println!("[DEBUG] exec_blocks_and_get_current_tester: head = {:?}", head);
        // assert_eq!(head, blocks.last().unwrap().canonical_root());

        Ok(self)
    }

    pub fn new(case: &MutatorForkChoiceTest<E>, spec: ChainSpec) -> Result<Self, Error> {
        // println!("[RUN] Build Tester");
        let genesis_time = case.anchor_state.genesis_time();

        if case.anchor_state.slot() != spec.genesis_slot {
            // I would hope that future fork-choice tests would start from a non-genesis anchors,
            // however at the time of writing, none do. I think it would be quite easy to do
            // non-genesis anchors via a weak-subjectivity/checkpoint start.
            //
            // Whilst those tests don't exist, we'll avoid adding checkpoint start complexity to the
            // `BeaconChainHarness` and create a hard failure so we can deal with it then.
            return Err(Error::FailedToParseTest(
                "anchor state is not a genesis state".into(),
            ));
        }

        // let keypairs = types::test_utils::generate_deterministic_keypairs::generate_deterministic_keypairs(256);
        // for keys in keypairs{
        //     println!("DetValidators PubKey: {:?}", keys.pk);
        // }


        let harness = BeaconChainHarness::builder(E::default())
            .spec(spec.clone())
            .keypairs(types::test_utils::load_keypairs_from_yaml(
                utils::get_workspace_dir().join("anchor_state").join("keys.yaml")
            ).unwrap())
            .genesis_state_ephemeral_store(case.anchor_state.clone())
            .mock_execution_layer()
            .recalculate_fork_times_with_genesis(0)
            .mock_execution_layer_all_payloads_valid()
            .build();

        // Drop any blocks that might be loaded in the mock execution layer. Some of these tests
        // will provide their own blocks and we want to start from a clean state.
        harness
            .mock_execution_layer
            .as_ref()
            .unwrap()
            .server
            .drop_all_blocks();

        assert_eq!(
            harness.chain.slot_clock.genesis_duration().as_secs(),
            genesis_time
        );

        Ok(Self { harness, spec })
    }

    fn tick_to_slot(&self, tick: u64) -> Result<Slot, Error> {
        let genesis_time = self.harness.chain.slot_clock.genesis_duration().as_secs();
        let since_genesis = tick
            .checked_sub(genesis_time)
            .ok_or_else(|| Error::FailedToParseTest("tick is prior to genesis".into()))?;
        let slots_since_genesis = since_genesis / self.spec.seconds_per_slot;
        Ok(self.spec.genesis_slot + slots_since_genesis)
    }

    fn block_on_dangerous<F: Future>(&self, future: F) -> Result<F::Output, Error> {
        self.harness
            .chain
            .task_executor
            .clone()
            .block_on_dangerous(future, "ef_tests_block_on")
            .ok_or_else(|| Error::InternalError("runtime shutdown".into()))
    }

    fn find_head(&self) -> Result<CachedHead<E>, Error> {
        let chain = self.harness.chain.clone();
        self.block_on_dangerous(chain.recompute_head_at_current_slot())?;
        Ok(self.harness.chain.canonical_head.cached_head())
    }

    pub fn set_tick(&self, tick: u64) {
        self.harness
            .chain
            .slot_clock
            .set_current_time(Duration::from_secs(tick));

        // Compute the slot time manually to ensure the slot clock is correct.
        let slot = self.tick_to_slot(tick).unwrap();
        assert_eq!(slot, self.harness.chain.slot().unwrap());

        self.harness
            .chain
            .canonical_head
            .fork_choice_write_lock()
            .update_time(slot, &self.spec)
            .unwrap();
    }

    pub fn process_block(&self, block: SignedBeaconBlock<E>, valid: bool) -> Result<(), Error> {
        let block_root = block.canonical_root();
        let block = Arc::new(block);
        let result = self.block_on_dangerous(self.harness.chain.process_block(
            block_root,
            block.clone(),
            CountUnrealized::False,
            NotifyExecutionLayer::Yes,
        ))?;
        if result.is_ok() != valid {
            return Err(Error::DidntFail(format!(
                "block with root {} was valid={} whilst test expects valid={}. result: {:?}",
                block_root,
                result.is_ok(),
                valid,
                result
            )));
        }

        // Apply invalid blocks directly against the fork choice `on_block` function. This ensures
        // that the block is being rejected by `on_block`, not just some upstream block processing
        // function.
        if !valid {
            // A missing parent block whilst `valid == false` means the test should pass.
            if let Some(parent_block) = self
                .harness
                .chain
                .get_blinded_block(&block.parent_root())
                .unwrap()
            {
                let parent_state_root = parent_block.state_root();
                let mut state = self
                    .harness
                    .chain
                    .get_state(&parent_state_root, Some(parent_block.slot()))
                    .unwrap()
                    .unwrap();

                complete_state_advance(
                    &mut state,
                    Some(parent_state_root),
                    block.slot(),
                    &self.harness.chain.spec,
                )
                .unwrap();

                let block_delay = self
                    .harness
                    .chain
                    .slot_clock
                    .seconds_from_current_slot_start(self.spec.seconds_per_slot)
                    .unwrap();

                let result = self
                    .harness
                    .chain
                    .canonical_head
                    .fork_choice_write_lock()
                    .on_block(
                        self.harness.chain.slot().unwrap(),
                        block.message(),
                        block_root,
                        block_delay,
                        &state,
                        PayloadVerificationStatus::Irrelevant,
                        &self.harness.chain.spec,
                        self.harness.chain.config.count_unrealized.into(),
                    );

                if result.is_ok() {
                    return Err(Error::DidntFail(format!(
                        "block with root {} should fail on_block",
                        block_root,
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn process_attestation(&self, attestation: &Attestation<E>) -> Result<(), Error> {
        let (indexed_attestation, _) =
            obtain_indexed_attestation_and_committees_per_slot(&self.harness.chain, attestation)
                .map_err(|e| {
                    Error::InternalError(format!("attestation indexing failed with {:?}", e))
                })?;
        let verified_attestation: ManuallyVerifiedAttestation<EphemeralHarnessType<E>> =
            ManuallyVerifiedAttestation {
                attestation,
                indexed_attestation,
            };

        self.harness
            .chain
            .apply_attestation_to_fork_choice(&verified_attestation)
            .map_err(|e| Error::InternalError(format!("attestation import failed with {:?}", e)))
    }

    pub fn process_attester_slashing(&self, attester_slashing: &AttesterSlashing<E>) {
        self.harness
            .chain
            .canonical_head
            .fork_choice_write_lock()
            .on_attester_slashing(attester_slashing)
    }

    pub fn process_pow_block(&self, pow_block: &PowBlock) {
        let el = self.harness.mock_execution_layer.as_ref().unwrap();

        // The EF tests don't supply a block number. Our mock execution layer is fine with duplicate
        // block numbers for the purposes of this test.
        let block_number = 0;

        el.server.insert_pow_block(
            block_number,
            pow_block.block_hash,
            pow_block.parent_hash,
            pow_block.total_difficulty,
        );
    }
}


/// An attestation that is not verified in the `BeaconChain` sense, but verified-enough for these
/// tests.
///
/// The `BeaconChain` verification is not appropriate since these tests use `Attestation`s with
/// multiple participating validators. Therefore, they are neither aggregated or unaggregated
/// attestations.
pub struct ManuallyVerifiedAttestation<'a, T: BeaconChainTypes> {
    #[allow(dead_code)]
    attestation: &'a Attestation<T::EthSpec>,
    indexed_attestation: IndexedAttestation<T::EthSpec>,
}

impl<'a, T: BeaconChainTypes> VerifiedAttestation<T> for ManuallyVerifiedAttestation<'a, T> {
    fn attestation(&self) -> &Attestation<T::EthSpec> {
        self.attestation
    }

    fn indexed_attestation(&self) -> &IndexedAttestation<T::EthSpec> {
        &self.indexed_attestation
    }
}
