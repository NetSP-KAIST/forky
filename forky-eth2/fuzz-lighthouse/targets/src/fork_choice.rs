use std::path::{Path};
use std::fs::File;
use std::io::{Write};
use std::fmt::Debug;
use ::fork_choice::{PayloadVerificationStatus};
use beacon_chain::slot_clock::SlotClock;
use beacon_chain::{
    attestation_verification::{
        obtain_indexed_attestation_and_committees_per_slot, VerifiedAttestation,
    },
    test_utils::{BeaconChainHarness, EphemeralHarnessType},
    BeaconChainTypes, CachedHead, CountUnrealized, NotifyExecutionLayer,
};
use execution_layer::{json_structures::JsonPayloadStatusV1Status, PayloadStatusV1};
use serde::Deserialize;
use ssz_derive::Decode;
use state_processing::state_advance::complete_state_advance;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use types::{
    Attestation, AttesterSlashing, BeaconBlock, BeaconState, Checkpoint, EthSpec,
    ExecutionBlockHash, ForkName, Hash256, IndexedAttestation, SignedBeaconBlock, Slot, Uint256,
    ChainSpec, 
};

use tree_hash::TreeHash;
use std::collections::HashMap;

use sha1::{Sha1, Digest};
use crate::utils::{*, self};

#[derive(Default, Debug, PartialEq, Clone, Deserialize, Decode)]
#[serde(deny_unknown_fields)]
pub struct PowBlock {
    pub block_hash: ExecutionBlockHash,
    pub parent_hash: ExecutionBlockHash,
    pub total_difficulty: Uint256,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Head {
    pub slot: Slot,
    pub root: Hash256,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    pub head: Option<Head>,
    pub time: Option<u64>,
    pub genesis_time: Option<u64>,
    pub justified_checkpoint: Option<Checkpoint>,
    pub justified_checkpoint_root: Option<Hash256>,
    pub finalized_checkpoint: Option<Checkpoint>,
    pub best_justified_checkpoint: Option<Checkpoint>,
    pub u_justified_checkpoint: Option<Checkpoint>,
    pub u_finalized_checkpoint: Option<Checkpoint>,
    pub proposer_boost_root: Option<Hash256>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadStatus {
    status: JsonPayloadStatusV1Status,
    latest_valid_hash: Option<ExecutionBlockHash>,
    validation_error: Option<String>,
}

impl From<PayloadStatus> for PayloadStatusV1 {
    fn from(status: PayloadStatus) -> Self {
        PayloadStatusV1 {
            status: status.status.into(),
            latest_valid_hash: status.latest_valid_hash,
            validation_error: status.validation_error,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Step<B, A, AS, P> {
    Tick {
        tick: u64,
    },  
    Block {
        block: B,
    },
    InvalidBlockCandidate {
        block: B,
        valid: bool,
    },
    Attestation {
        attestation: A,
    },
    AttesterSlashing {
        attester_slashing: AS,
    },
    PowBlock {
        pow_block: P,
    },
    OnPayloadInfo {
        block_hash: ExecutionBlockHash,
        payload_status: PayloadStatus,
    },
    Checks {
        checks: Box<Checks>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForkChoiceTest<E: EthSpec> {
    pub testcase_name: String,
    pub anchor_state: BeaconState<E>,
    pub anchor_block: BeaconBlock<E>,
    #[allow(clippy::type_complexity)]
    pub steps: Vec<Step<SignedBeaconBlock<E>, Attestation<E>, AttesterSlashing<E>, PowBlock>>,
}

impl<E: EthSpec> ForkChoiceTest<E> {
    /// SHA1 hash String of the test case.
    fn get_testcase_name(testcase: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(testcase);
        let hash = hasher.finalize();
        println!("[EXECUTE] Loading Testcase {:x}", hash);
        return format!("{:x}", hash);
    }

    pub fn load_testcase_from_yaml(testcase_yaml_bin: &[u8], 
            fork_name: ForkName, 
            path: &Path)   
        -> Result<Self, Error> {
        let spec = &testing_spec::<E>(fork_name);
        let testcase_name = Self::get_testcase_name(testcase_yaml_bin);

        let path_block = path.join("blocks");
        // println!("[DEBUG] path_block: {:?}", path_block);
        let path_att = path.join("attestations");
        // println!("[DEBUG] path_att: {:?}", path_att);
        let path_anchor_blk = path.join("anchor_block");
        // println!("[DEBUG] path_anchor_blk: {:?}", path_anchor_blk);
        let path_anchor_state = path.join("anchor_state");
        // println!("[DEBUG] path_anchor_state: {:?}", path_anchor_state);

        let steps: Vec<Step<String, String, String, String>> = 
            yaml_decode_bin(testcase_yaml_bin)?;

        let steps = steps
            .into_iter()
            .map(|step| match step {
                Step::Tick { tick } => Ok(Step::Tick { tick }),
                Step::Block { block } => {
                    // println!("[DEBUG] BLOCK");
                    ssz_decode_file_with(&path_block.join(format!("{}.ssz_snappy", block)), |bytes| {
                        SignedBeaconBlock::from_ssz_bytes(bytes, spec)
                    })
                    .map(|block| Step::Block { block })
                }
                Step::InvalidBlockCandidate { block, valid } => {
                    ssz_decode_file_with(&path.join(format!("{}.ssz_snappy", block)), |bytes| {
                        SignedBeaconBlock::from_ssz_bytes(bytes, spec)
                    })
                    .map(|block| Step::InvalidBlockCandidate { block, valid })
                }
                Step::Attestation { attestation } => {
                    // println!("[DEBUG] ATTESTATION");
                    ssz_decode_file(&path_att.join(format!("{}.ssz_snappy", attestation)))
                        .map(|attestation| Step::Attestation { attestation })
                }
                Step::AttesterSlashing { attester_slashing } => {
                    // println!("[DEBUG] ATTESTER SLASHING");
                    ssz_decode_file(&path.join(format!("{}.ssz_snappy", attester_slashing)))
                        .map(|attester_slashing| Step::AttesterSlashing { attester_slashing })
                }
                Step::PowBlock { pow_block } => {
                    // println!("[DEBUG] POW BLOCK");
                    ssz_decode_file(&path.join(format!("{}.ssz_snappy", pow_block)))
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
            })
            .collect::<Result<_, _>>()?;
        let anchor_state = ssz_decode_state(&path_anchor_state.join("anchor_state.ssz_snappy"), spec)?;
        let anchor_block = ssz_decode_file_with(&&path_anchor_blk.join("anchor_block.ssz_snappy"), |bytes| {
            BeaconBlock::from_ssz_bytes(bytes, spec)
        })?;

        Ok(Self {
            testcase_name,
            anchor_state,
            anchor_block,
            steps,
        })
    }

    pub fn run(testcase: ForkChoiceTest<E>, fork_name: ForkName) 
    -> Result<Vec<utils::ReorgType>, Error> {
        println!("[EXECUTE] Step = Block");
        let spec = testing_spec::<E>(fork_name);
        let tester = Tester::new(&testcase, spec)?;

        let mut steps_with_checks: Vec<Step<SignedBeaconBlock<E>, Attestation<E>, AttesterSlashing<E>, PowBlock>> = Vec::new();
        let mut reorg_counter: u32 = 0;
        let mut reorg_vectors: Vec<utils::ReorgType> = Vec::new();
        let mut blocks = HashMap::new();
        for step in &testcase.steps {
            steps_with_checks.push(step.clone());
            match step {
                Step::Tick { tick } => tester.set_tick(*tick),
                Step::Block { block } => {
                    let old_cached_head = tester.find_head().unwrap();
                    let old_head_weight = tester.get_weight(old_cached_head.head_block_root());
                    println!("\tStep = Block: {:?}", block.tree_hash_root());
                    println!("\t\t Block Hash: {:?}", block.message().tree_hash_root());
                    blocks.insert(block.canonical_root(), block.clone());
                    tester.process_block(block.clone(), true).unwrap();
                    println!("\t\t\tProcessed: state root: {:?}", tester.harness.chain.wall_clock_state().unwrap().canonical_root());
                    // println!("\t\t\t[DEBUG] attestations: {:?}", block.message().body().attestations());

                    let new_cached_head = tester.find_head().unwrap();
                    let new_head_weight = tester.get_weight(new_cached_head.head_block_root());
                    let is_reorg = utils::detect_reorg(
                        &old_cached_head.snapshot.beacon_state, 
                        old_cached_head.head_block_root(), 
                        &new_cached_head.snapshot.beacon_state, 
                        new_cached_head.head_block_root());
                    if is_reorg {
                        reorg_counter += 1;
                        println!("[DEBUG] Reorg detected!");
                        println!("[DEBUG] Old head: {:?}, weight: {:?}", old_cached_head.head_block_root(), old_head_weight);
                        println!("[DEBUG] New head: {:?}, weight: {:?}", new_cached_head.head_block_root(), new_head_weight);
                        let (ancestor_slot, ancestor_root) = utils::common_ancestor(
                            &old_cached_head.snapshot.beacon_state, 
                            old_cached_head.head_block_root(), 
                            &new_cached_head.snapshot.beacon_state, 
                            new_cached_head.head_block_root(),
                            &testing_spec::<E>(fork_name)
                        );
                        let n_replacing_blocks = utils::get_nblock_to_ancestor(blocks.clone(), new_cached_head.head_block_root(), ancestor_root);
                        let n_replaced_blocks = utils::get_nblock_to_ancestor(blocks.clone(), old_cached_head.head_block_root(), ancestor_root);
                        let n_replacing_slots = new_cached_head.head_slot().saturating_sub(ancestor_slot).as_u64();
                        let n_replaced_slots = old_cached_head.head_slot().saturating_sub(ancestor_slot).as_u64();
                        let weight_gap_exists = new_head_weight != old_head_weight;
                        let is_boosted = tester.harness.chain.canonical_head.fork_choice_read_lock().proposer_boost_root() != block.canonical_root();
                        let epoch_gap_exists = (new_cached_head.head_slot().epoch(E::slots_per_epoch()).as_u64() as i64 - old_cached_head.head_slot().epoch(E::slots_per_epoch()).as_u64() as i64);
                        let justified_epoch_gap =
                            new_cached_head.justified_checkpoint().epoch.as_u64() as i64
                            - old_cached_head.justified_checkpoint().epoch.as_u64() as i64;
                        
                        let reorg_type = ReorgType::new(
                            n_replacing_blocks,
                            n_replaced_blocks,
                            n_replacing_slots as u32,
                            n_replaced_slots as u32,
                            weight_gap_exists,
                            is_boosted,
                            epoch_gap_exists as i32,
                            justified_epoch_gap as i32,
                            reorg_counter,
                        );
                        println!("Reorg type: {:?}", reorg_type);
                        reorg_vectors.push(reorg_type);
                    }

                    let head = tester.find_head().unwrap();
                    steps_with_checks.push(Step::Checks {
                        checks: Box::new(Checks {
                            head: Some(Head{slot: tester.harness.chain.canonical_head.cached_head().head_slot(),
                                            root: tester.harness.chain.canonical_head.cached_head().head_block_root()} ),
                            time: None,
                            genesis_time: Some(tester.harness.chain.genesis_time),
                            justified_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().justified_checkpoint()),
                            justified_checkpoint_root: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().justified_checkpoint().root),
                            finalized_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().finalized_checkpoint()),
                            best_justified_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().best_justified_checkpoint()),
                            u_justified_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().unrealized_justified_checkpoint()),
                            u_finalized_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().unrealized_finalized_checkpoint()),
                            proposer_boost_root: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().proposer_boost_root()),
                        }),
                    })
                },
                Step::InvalidBlockCandidate { block, valid } => {
                    println!("\tStep = InvalidBlockCandidate: {:?}", block.tree_hash_root());
                    println!("\t\t Block Hash: {:?}", block.message().tree_hash_root());
                    tester.process_block(block.clone(), *valid).unwrap();
                    println!("\t\t\tProcessed: state root: {:?}", tester.harness.chain.wall_clock_state().unwrap().canonical_root());

                    let head = tester.find_head().unwrap();
                    steps_with_checks.push(Step::Checks {
                        checks: Box::new(Checks {
                            head: Some(Head{slot: tester.harness.chain.canonical_head.cached_head().head_slot(),
                                            root: tester.harness.chain.canonical_head.cached_head().head_block_root()} ),
                            time: None,
                            genesis_time: Some(tester.harness.chain.genesis_time),
                            justified_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().justified_checkpoint()),
                            justified_checkpoint_root: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().justified_checkpoint().root),
                            finalized_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().finalized_checkpoint()),
                            best_justified_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().best_justified_checkpoint()),
                            u_justified_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().unrealized_justified_checkpoint()),
                            u_finalized_checkpoint: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().unrealized_finalized_checkpoint()),
                            proposer_boost_root: Some(tester.harness.chain.canonical_head.fork_choice_read_lock().proposer_boost_root()),
                        }),
                    })
                },
                Step::Attestation { attestation } => {
                    println!("\tStep = Attestation: {:?}", attestation.tree_hash_root());
                    println!("\t\tAttestation is attesting block: {:?}", attestation.data.beacon_block_root);
                    // println!("[DEBUG] Attestation aggregation bit: {:?}", attestation.aggregation_bits);
                    tester.process_attestation(attestation)?
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
                Step::Checks { checks } => {
                    // Currently do nothing during checks.
                    let Checks {
                        head,
                        time,
                        genesis_time,
                        justified_checkpoint,
                        justified_checkpoint_root,
                        finalized_checkpoint,
                        best_justified_checkpoint,
                        u_justified_checkpoint,
                        u_finalized_checkpoint,
                        proposer_boost_root,
                    } = checks.as_ref();
                }
            }
            // tester.harness.chain.recompute_head_at_current_slot();
        }

        let yaml_with_checks = yaml_encode_with_steps(steps_with_checks);
        let path_testcase = get_workspace_dir().join("test_cases_with_check").join(Self::get_testcase_name(yaml_with_checks.as_bytes()));
        let mut file_testcase = File::create(path_testcase).unwrap();

        file_testcase.write(yaml_with_checks.as_bytes());
        println!("[DEBUG] Reorg counter: {:?}", reorg_counter);
        utils::write_reorg_count_report(reorg_counter);

        Ok(reorg_vectors)
    }
}


/// A testing rig used to execute a test case.
struct Tester<E: EthSpec> {
    harness: BeaconChainHarness<EphemeralHarnessType<E>>,
    spec: ChainSpec,
}

impl<E: EthSpec> Tester<E> {
    pub fn new(case: &ForkChoiceTest<E>, spec: ChainSpec) -> Result<Self, Error> {
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

        let harness = BeaconChainHarness::builder(E::default())
            .spec(spec.clone())
            .keypairs(vec![])
            .genesis_state_ephemeral_store(case.anchor_state.clone())
            .mock_execution_layer()
            .recalculate_fork_times_with_genesis(0)
            .mock_execution_layer_all_payloads_valid()
            .build();

        if harness.chain.genesis_block_root != case.anchor_block.canonical_root() {
            // This check will need to be removed if/when the fork-choice tests use a non-genesis
            // anchor state.
            return Err(Error::FailedToParseTest(
                "anchor block differs from locally-generated genesis block".into(),
            ));
        }

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

    pub fn get_weight(&self, root: Hash256) -> u64 {
        self.harness
            .chain
            .canonical_head
            .fork_choice_read_lock()
            .get_block_weight(&root)
            .unwrap()
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
