use fs2::FileExt;
use snap::raw::Decoder;
use tree_hash::TreeHash;
use std::collections::HashMap;
use std::fs::{self};
use std::io::Write;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::str;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use types::{BeaconState, EthSpec, ChainSpec, ForkName, SignedBeaconBlock, Attestation, AttesterSlashing, Hash256, Slot};

use crate::fork_choice::{Step, Checks, PowBlock};

pub fn testing_spec<E: EthSpec>(fork_name: ForkName) -> ChainSpec {
    fork_name.make_genesis_spec(E::default_spec())
}

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    /// The value in the test didn't match our value.
    NotEqual(String),
    /// The test specified a failure and we did not experience one.
    DidntFail(String),
    /// Failed to parse the test (internal error).
    FailedToParseTest(String),
    /// Test case contained invalid BLS data.
    InvalidBLSInput(String),
    /// Skipped the test because the BLS setting was mismatched.
    SkippedBls,
    /// Skipped the test because it's known to fail.
    SkippedKnownFailure,
    /// The test failed due to some internal error preventing the test from running.
    InternalError(String),
}

impl Error {
    pub fn name(&self) -> &str {
        match self {
            Error::NotEqual(_) => "NotEqual",
            Error::DidntFail(_) => "DidntFail",
            Error::FailedToParseTest(_) => "FailedToParseTest",
            Error::InvalidBLSInput(_) => "InvalidBLSInput",
            Error::SkippedBls => "SkippedBls",
            Error::SkippedKnownFailure => "SkippedKnownFailure",
            Error::InternalError(_) => "InternalError",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Error::NotEqual(m) => m.as_str(),
            Error::DidntFail(m) => m.as_str(),
            Error::FailedToParseTest(m) => m.as_str(),
            Error::InvalidBLSInput(m) => m.as_str(),
            Error::InternalError(m) => m.as_str(),
            _ => self.name(),
        }
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, Error::SkippedBls | Error::SkippedKnownFailure)
    }
}


/// See `log_file_access` for details.
const ACCESSED_FILE_LOG_FILENAME: &str = ".accessed_file_log.txt";

/// Writes `path` to a file that contains a log of all files accessed during testing.
///
/// That log file might later be used to ensure that all spec tests were accessed and none were
/// accidentally missed.
pub fn log_file_access<P: AsRef<Path>>(file_accessed: P) {
    let passed_test_list_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ACCESSED_FILE_LOG_FILENAME);

    let mut file = fs::File::options()
        .append(true)
        .create(true)
        .open(passed_test_list_path)
        .expect("should open file");

    file.lock_exclusive().expect("unable to lock file");

    writeln!(&mut file, "{:?}", file_accessed.as_ref()).expect("should write to file");

    file.unlock().expect("unable to unlock file");
}

pub fn yaml_decode<T: serde::de::DeserializeOwned>(string: &str) -> Result<T, Error> {
    serde_yaml::from_str(string).map_err(|e| Error::FailedToParseTest(format!("{:?}", e)))
}

pub fn yaml_decode_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Error> {
    log_file_access(path);
    fs::read_to_string(path)
        .map_err(|e| {
            Error::FailedToParseTest(format!("Unable to load {}: {:?}", path.display(), e))
        })
        .and_then(|s| yaml_decode(&s))
}

pub fn yaml_decode_bin<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T, Error> {
    // println!("[DBUG] CALL yaml_decode_bin");
    str::from_utf8(data)
        .map_err(|e| {
            Error::FailedToParseTest(format!("yaml_decode_bin: Unable to parse test case: {:?}", e))
        })
        .and_then(|s| yaml_decode(&s))
}

/// Decode a Snappy encoded file.
///
/// Files in the EF tests are unframed, so we need to use `snap::raw::Decoder`.
pub fn snappy_decode_file(path: &Path) -> Result<Vec<u8>, Error> {
    log_file_access(path);
    let bytes = fs::read(path).map_err(|e| {
        Error::FailedToParseTest(format!("Unable to load {}: {:?}", path.display(), e))
    })?;
    let mut decoder = Decoder::new();
    decoder.decompress_vec(&bytes).map_err(|e| {
        Error::FailedToParseTest(format!(
            "Error decoding snappy encoding for {}: {:?}",
            path.display(),
            e
        ))
    })
}

pub fn ssz_decode_file_with<T, F>(path: &Path, f: F) -> Result<T, Error>
where
    F: FnOnce(&[u8]) -> Result<T, ssz::DecodeError>,
{
    log_file_access(path);
    let bytes = snappy_decode_file(path)?;
    f(&bytes).map_err(|e| {
        match e {
            // NOTE: this is a bit hacky, but seemingly better than the alternatives
            ssz::DecodeError::BytesInvalid(message)
                if message.contains("Blst") || message.contains("Milagro") =>
            {
                Error::InvalidBLSInput(message)
            }
            e => Error::FailedToParseTest(format!(
                "Unable to parse SSZ at {}: {:?}",
                path.display(),
                e
            )),
        }
    })
}

pub fn ssz_decode_file<T: ssz::Decode>(path: &Path) -> Result<T, Error> {
    log_file_access(path);
    ssz_decode_file_with(path, T::from_ssz_bytes)
}

pub fn ssz_decode_state<E: EthSpec>(
    path: &Path,
    spec: &ChainSpec,
) -> Result<BeaconState<E>, Error> {
    log_file_access(path);
    ssz_decode_file_with(path, |bytes| BeaconState::from_ssz_bytes(bytes, spec))
}

pub fn get_workspace_dir() -> std::path::PathBuf {
    let p_ = env::var("CARGO_FC_DIR")
        .map(From::from)
        .or_else(|_| env::current_dir()).unwrap();
    
    p_.join("workspace")
}

pub fn yaml_encode_with_steps<E: EthSpec>(
    steps: Vec<Step<SignedBeaconBlock<E>, Attestation<E>, AttesterSlashing<E>, PowBlock>>) 
-> String {
let mut new_str: Vec::<String> = Vec::new();
for step in steps {
    // println!("[DEBUG] mutator step: {:?}", step);
    match step {
        // Step::Block { .. } => new_str.push(format!("block: {}", step)),
        Step::Tick { tick } => new_str.push(format!("- {{tick: {}}}", tick)),
        Step::Block { block } => new_str.push(format!("- {{block: block_{:?}}}", block.tree_hash_root())),
        Step::Attestation { attestation } => new_str.push(format!("- {{attestation: attestation_{:?}}}", attestation.tree_hash_root())),
        Step::AttesterSlashing { attester_slashing } => new_str.push(format!("- {{attester_slashing: {:?}}}", attester_slashing.tree_hash_root())),
        Step::PowBlock { pow_block } => new_str.push(format!("- {{pow_block: {:?}}}", pow_block.block_hash)),
        Step::Checks { checks } => {
            // println!("[DEBUG] Writing Checks in test case");
            let Checks{
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

            new_str.push(format!("- checks:"));
            new_str.push(format!("    head: {{slot: {}, root: '{:?}'}}", head.unwrap().slot, head.unwrap().root));
            // new_str.push(format!("    time: {}", time.unwrap()));
            // new_str.push(format!("    genesis_time: {}\n", genesis_time.unwrap()));
            new_str.push(format!("    justified_checkpoint: {{epoch: {}, root: '{:?}'}}", justified_checkpoint.unwrap().epoch, justified_checkpoint.unwrap().root));
            // new_str.push(format!("    justified_checkpoint_root: '{}'\n", justified_checkpoint_root.unwrap()));
            new_str.push(format!("    finalized_checkpoint: {{epoch: {}, root: '{:?}'}}", finalized_checkpoint.unwrap().epoch, finalized_checkpoint.unwrap().root));
            // new_str.push(format!("    best_justified_checkpoint: {{epoch: {}, root: '{}'}}\n", best_justified_checkpoint.unwrap().epoch, best_justified_checkpoint.unwrap().root));
            new_str.push(format!("    proposer_boost_root: '{:?}'", proposer_boost_root.unwrap()));
        }
        _ => unreachable!(),
    }
}

let encoded_yaml = new_str.join("\n");
encoded_yaml
}


#[derive(Debug, Clone, PartialEq)]
pub struct ReorgType {
    n_replacing_blocks: u32,
    n_replaced_blocks: u32,
    /* Note: n_votes makes too many distinct reorg types */ 
    // n_votes_in_replacing_branch: u32,
    // n_votes_in_replace_branch: u32,
    replacing_slot_distance: u32, 
    replaced_slot_distance: u32, 
    weight_gap_exists: bool, // if true, it means reorg triggered by vote, if not triggered block hash
    is_boosted: bool, // check if reorg affected by proposer boost
    /* Note:Requires too much cost to calculate */ 
    // n_blocks_replaced_but_now_replacing: u32,
    epoch_gap: i32, // if not 0, epoch changed
    justified_epoch_gap: i32, // May 0, if it change, it means unrealized justification reorg attack
    // u_justified_checkpoint_gap: i32, 
    n_th_reorg: u32,
}

impl ReorgType {
    pub fn new(
        n_replacing_blocks: u32,
        n_replaced_blocks: u32,
        /* Note: n_votes makes too many distinct reorg types */ 
        // n_votes_in_replacing_branch: u32,
        // n_votes_in_replace_branch: u32,
        replacing_slot_distance: u32, 
        replaced_slot_distance: u32, 
        weight_gap_exists: bool, // if true, it means reorg triggered by vote, if not triggered block hash
        is_boosted: bool, // check if reorg affected by proposer boost
        /* Note:Requires too much cost to calculate */ 
        // n_blocks_replaced_but_now_replacing: u32,
        epoch_gap: i32, // if not 0, epoch changed
        justified_epoch_gap: i32, // May 0, if it change, it means unrealized justification reorg attack
        // u_justified_checkpoint_gap: i32, // Hard to get
        n_th_reorg: u32,
    ) 
    -> ReorgType {
        let reorg = ReorgType {
            n_replacing_blocks,
            n_replaced_blocks,
            // n_votes_in_replacing_branch: 0,
            // n_votes_in_replace_branch: 0,
            replacing_slot_distance,
            replaced_slot_distance,
            weight_gap_exists,
            is_boosted,
            // n_blocks_replaced_but_now_replacing: 0,
            // is_epoch_changed: false,
            epoch_gap,
            justified_epoch_gap,
            // u_justified_checkpoint_gap,
            n_th_reorg,
        };
        reorg
    }
}

// Check lighthouse/beacon_node/beacon_chain/src/canonical_head.rs::detect_reorg()
pub fn  detect_reorg <E: EthSpec>(
    old_state: &BeaconState<E>,
    old_block_root: Hash256,
    new_state: &BeaconState<E>,
    new_block_root: Hash256,
) -> bool {
    // If nothing happened after block processing, no reorg
    if old_block_root == new_block_root {
        return false;
    }

    // println!("old_block_root:{:?}", old_block_root);
    // println!("new block root of old state slot: {:?}", new_state.get_block_root(old_state.slot()).unwrap());
    let is_reorg = new_state
        .get_block_root(old_state.slot())
        .map_or(true, |root| *root != old_block_root);

    if is_reorg {
        println!("[DEBUG] Reorg detected");
    }
    is_reorg
}

// Get the reorg starting point slot (common ancestor)
// Check lighthouse/beacon_node/beacon_chain/src/canonical_head.rs::find_reorg_slot()
pub fn common_ancestor <E: EthSpec>(
    old_state: &BeaconState<E>,
    old_block_root: Hash256,
    new_state: &BeaconState<E>,
    new_block_root: Hash256,
    spec: &ChainSpec,
) -> (Slot, Hash256) {
    let lowest_slot = std::cmp::min(new_state.slot(), old_state.slot());
    
    macro_rules! aligned_roots_iter {
        ($state: ident, $block_root: ident) => {
            std::iter::once(Ok(($state.slot(), $block_root)))
                .chain($state.rev_iter_block_roots(spec))
                .skip_while(|result| {
                    result
                        .as_ref()
                        .map_or(false, |(slot, _)| *slot > lowest_slot)
                })
        };
    }

    let mut new_roots = aligned_roots_iter!(new_state, new_block_root);
    let mut old_roots = aligned_roots_iter!(old_state, old_block_root);

    // Whilst *both* of the iterators are still returning values, try and find a common
    // ancestor between them.
    while let (Some(old), Some(new)) = (old_roots.next(), new_roots.next()) {
        let (old_slot, old_root) = old.unwrap();
        let (new_slot, new_root) = new.unwrap();

        // Sanity check to detect programming errors.
        if old_slot != new_slot {
            println!("Err: old_slot: {}, new_slot: {}", old_slot, new_slot);
        }

        if old_root == new_root {
            // A common ancestor has been found.
            return (old_slot, old_root);
        }
    }

    // If no common ancestor is found, declare that the re-org happened at the previous finalized slot.
    let finalized_slot = old_state
        .finalized_checkpoint()
        .epoch
        .start_slot(E::slots_per_epoch());
    let finalized_root = old_state.get_block_root(finalized_slot).unwrap();
    (finalized_slot, *finalized_root)
}

pub fn get_nblock_to_ancestor<E: EthSpec>(
    blocks: HashMap<Hash256, SignedBeaconBlock<E>>,
    block_root: Hash256,
    ancestor_root: Hash256,
) -> u32 {
    let mut nblock: u32 = 0;
    let mut root = block_root;
    while root != ancestor_root {
        let block = match blocks.get(&root) {
            Some(block) => block,
            None => {
                println!("Err: block not found");
                return 0;
            }
        };
        root = block.parent_root();
        nblock += 1;
    }
    nblock
}

pub fn write_reorg_count_report (
    counter: u32
) -> Result<(), Error>{
    let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(get_workspace_dir().join("reports").join("reorg_count.txt"))
            .unwrap();

    if let Err(e) = writeln!(file, "{}", counter) {
        eprintln!("Couldn't write to file: {}", e);
    }
    Ok(())
}

pub fn write_reorg_type_report (
    iter: u32,
    counter: u32,
    reorg_vectors: Vec<ReorgType>
) -> Result<(), Error>{
    let now_ = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let now = now_.as_secs();
    let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(get_workspace_dir().join("reports").join("new_reorg.txt"))
            .unwrap();

    if reorg_vectors.is_empty() {
        if let Err(e) = writeln!(file, "{}, {}, {}, 0, \"\", \"\", \"\", \"\", \"\", \"\", \"\", \"\", \"\" ", now, iter, counter) {
            eprintln!("Couldn't write to file: {}", e);
        }
        return Ok(())
    }
    // n_replacing_blocks: u32,
    // n_replaced_blocks: u32,
    // replacing_slot_distance: u32, 
    // replaced_slot_distance: u32, 
    // weight_gap_exists: bool, // if true, it means reorg triggered by vote, if not triggered block hash
    // is_boosted: bool, // check if reorg affected by proposer boost
    // epoch_gap: i32, // if not 0, epoch changed
    // justified_epoch_gap: i32, // May 0, if it change, it means unrealized justification reorg attack
    // n_th_reorg: u32,
    let n_reorg = reorg_vectors.len();
    for reorg in reorg_vectors {
        let weight_gap_exists_num = if reorg.weight_gap_exists {1} else {0};
        let is_boosted_num = if reorg.is_boosted {1} else {0};
        if let Err(e) = writeln!(file, "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}", 
        now, iter, counter, n_reorg,
            reorg.n_replacing_blocks, 
            reorg.n_replaced_blocks,
            reorg.replacing_slot_distance,
            reorg.replaced_slot_distance,   
            weight_gap_exists_num,
            is_boosted_num,
            reorg.epoch_gap,
            reorg.justified_epoch_gap,
            reorg.n_th_reorg) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }
    Ok(())
}