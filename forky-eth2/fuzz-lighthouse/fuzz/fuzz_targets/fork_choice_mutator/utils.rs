use std::str;
extern crate targets;
use targets::fork_choice::{Step, Checks, Head,};
use targets::utils;
use std::path::*;
use std::fs;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::env;
use rand::{Rng, thread_rng, seq::SliceRandom};

use std::fs::{File};
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io;
use types::{Hash256, Attestation, BeaconState, AttesterSlashing,
        SignedBeaconBlock, BeaconBlock, EthSpec, RelativeEpoch,
        MainnetEthSpec, ChainSpec, ForkName, Checkpoint, Slot, Epoch};

use std::io::{BufWriter, Write, Read};
use ssz::{Encode, ssz_encode, Decode};
use ssz_derive::Decode;
use futures::executor::block_on;
use snap::write::FrameEncoder;
use tree_hash::TreeHash;
use tree_hash_derive::TreeHash;

use sha1::{Digest, Sha1};
use hex;
use bls::{Keypair, PublicKey, SecretKey};
use num_bigint::BigUint;
use eth2_hashing::hash;

use state_processing::{
    per_block_processing, BlockSignatureStrategy, BlockProcessingError, ConsensusContext, VerifyBlockRoot, per_slot_processing,
};
const SEC_PER_SLOT: u64 = 12;
const SLOT_PER_EPOCH: u64 = 32;


// Check epoch mutation enabled
pub fn check_mut_epoch_enabled() -> bool {
    env::var("ENABLE_EPOCH_MUTATION")
        .map_or_else(
            |e| {
                println!("[FORKY] DO NOT CONDUCT EPOCH MUTATION: {}", e);
                false
            },
            |val| {
                if val == "1" {
                    println!("[FORKY] EPOCH MUTATION ENABLED");
                    true
                } else {
                    println!("[FORKY] DO NOT CONDUCT EPOCH MUTATION: ENABLE_EPOCH_MUTATION = {}", val);
                    false
                }
            },
        )
}

// Force save to corpus
pub fn write_testcase_to_corpus(
    data: &[u8],
) {
    let content = match str::from_utf8(data) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    }.to_string();

    // Hash the content using SHA1
    let mut hasher = Sha1::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    // Generate a filename from the hash
    let mut filename = String::new();
    for byte in hash.iter() {
        filename.push_str(&format!("{:02x}", byte));
    }
    let path = get_workspace_dir().join("corpus").join(filename.clone());
    println!("[FORKY] SAVING to Corpus: {:?}", filename);

    let mut file = File::create(path).unwrap();
    file.write(content.as_bytes()).unwrap();
}

pub fn remove_testcase_from_corpus(
    data: &[u8],
)  {
    let content = match str::from_utf8(data) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    }.to_string();

    // Hash the content using SHA1
    let mut hasher = Sha1::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    // Generate a filename from the hash
    let mut filename = String::new();
    for byte in hash.iter() {
        filename.push_str(&format!("{:02x}", byte));
    }

    let path = get_workspace_dir().join("corpus").join(filename.clone());
    println!("[FORKY] Target: {:?}", path);
    let _ = fs::remove_file(path).expect("File delete failed");
    println!("[FORKY] Remove {:?} from Corpus", filename);

}

pub fn write_testcase_to_file(
    content: String,
) -> std::io::Result<()> {
    // Hash the content using SHA1
    let mut hasher = Sha1::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    // Generate a filename from the hash
    let mut filename = String::new();
    for byte in hash.iter() {
        filename.push_str(&format!("{:02x}", byte));
    }
    let path = get_workspace_dir().join("test_cases").join(filename.clone());
    println!("[DEBUG] SAVING: {:?}", filename);

    let mut file = File::create(path).unwrap();
    file.write(content.as_bytes()).unwrap();
    Ok(())
}

pub fn write_block_to_file(
    block: SignedBeaconBlock<MainnetEthSpec>,
) -> std::io::Result<()> {
    let hashed_name = block.tree_hash_root();
    println!("\t[SAVING] {:?}", hashed_name);
    let path = get_workspace_dir().join("blocks").join(format!("block_{:?}.ssz_snappy", hashed_name));
    println!("\t[SAVING] Write block to file: {:?}", path);
    let ssz = block.as_ssz_bytes();
    // Open the output file
    let mut file = File::create(path).unwrap();
    // Compress the SSZ bytes
    let mut snappy = {
        let mut encoder = FrameEncoder::new(Vec::new());
        encoder.write_all(&ssz).unwrap();
        encoder.into_inner().unwrap()
    };
    // println!("\n");
    // for b in snappy.clone() {
    //     print!("{:02x} ", b);
    // }

    // Remove the snappy header
    file.write_all(&snappy[18..]).unwrap();

    Ok(())
}

pub fn write_attestation_to_file(
    attestation: Attestation<MainnetEthSpec>,
) -> std::io::Result<()> {
    let hashed_name = attestation.tree_hash_root();
    println!("\t[SAVING] {:?}", hashed_name);
    let path = get_workspace_dir().join("attestations").join(format!("attestation_{:?}.ssz_snappy", hashed_name));
    println!("\t[SAVING] Write attestation to file: {:?}", path);
    let ssz = attestation.as_ssz_bytes();
    // Open the output file
    let mut file = File::create(path).unwrap();
    // Compress the SSZ bytes
    let mut snappy = {
        let mut encoder = FrameEncoder::new(Vec::new());
        encoder.write_all(&ssz).unwrap();
        encoder.into_inner().unwrap()
    };

    // Remove the snappy header
    file.write_all(&snappy[18..]).unwrap();

    Ok(())
}

/// Read the contents from file path
pub fn read_raw_data(
        data_name: String, 
        data_type: String) 
    -> Result<Vec<u8>, io::Error> {
		// println!("[DEBUG] CALL: read_contents_from_path");
	    let mut buffer: Vec<u8> = Vec::new();
	    let file_path = get_workspace_dir().join(data_type).join(format!("{}.ssz_snappy", data_name));

	    let mut file = File::open(file_path)?;
	    file.read_to_end(&mut buffer)?;
	    // We force to close the file
	    drop(file);
	    Ok(buffer)
}

pub fn check_step_sorted(
    steps: Vec<Step<String, String, String, String>>
) -> bool {
    let mut tick = 0;
    for step in steps {
        match step {
            Step::Tick { tick: t } => {
                if t < tick {
                    return false;
                }
                tick = t;
            },
            _ => {},
        }
    }
    true
}

pub fn sort_step(
    steps: Vec<Step<String, String, String, String>>
) -> Vec<Step<String, String, String, String>> {
    let mut current_tick = 0;
    let mut ticks_with_data = HashMap::new();
    let mut new_steps: Vec<Step<String, String, String, String>> = Vec::new();
    for step in steps {
        match step {
            Step::Tick { tick } => {
                if tick > current_tick {
                    current_tick = tick;
                    let mut data: Vec<Step<String, String, String, String>> = Vec::new();
                    ticks_with_data.insert(current_tick, data);
                }
            },
            Step::Block {block} => {
                let data = ticks_with_data.get_mut(&current_tick).unwrap();
                data.push(Step::Block { block });
            },
            Step::Attestation {attestation} => {
                let data = ticks_with_data.get_mut(&current_tick).unwrap();
                data.push(Step::Attestation { attestation });
            },
            _ => unreachable!(),
        }
    }

    let keys: Vec<u64> = ticks_with_data.keys().cloned().collect();;
    let mut sorted_keys: Vec<u64> = keys.clone();
    sorted_keys.sort();
    for key in sorted_keys {
        new_steps.push(Step::Tick { tick: key });
        let data = ticks_with_data.get_mut(&key).unwrap();
        for step in data {
            new_steps.push(step.clone());
        }
    }
    new_steps
}

pub fn yaml_encode_with_steps(
        steps: Vec<Step<String, String, String, String>>) 
    -> String {
    let mut new_str: Vec::<String> = Vec::new();
    for step in steps {
        // println!("[DEBUG] mutator step: {:?}", step);
        match step {
            // Step::Block { .. } => new_str.push(format!("block: {}", step)),
            Step::Tick { tick } => new_str.push(format!("- {{tick: {}}}", tick)),
            Step::Block { block } => new_str.push(format!("- {{block: {}}}", block)),
            Step::Attestation { attestation } => new_str.push(format!("- {{attestation: {}}}", attestation)),
            Step::AttesterSlashing { attester_slashing } => new_str.push(format!("- {{attester_slashing: {}}}", attester_slashing)),
            Step::PowBlock { pow_block } => new_str.push(format!("- {{pow_block: {}}}", pow_block)),
            Step::Checks { checks } => {
                // println!("[DEBUG] Writing Checks in test case");
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

                new_str.push(format!("- checks:"));
                new_str.push(format!("    head: {{slot: {}, root: '{:?}'}}", head.unwrap().slot, head.unwrap().root));
                new_str.push(format!("    time: {}", time.unwrap()));
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

pub fn get_workspace_dir() -> std::path::PathBuf {
    let p_ = env::var("CARGO_FC_DIR")
        .map(From::from)
        .or_else(|_| env::current_dir()).unwrap();
    let workspace_path = p_.join("workspace");
    workspace_path
}

/// Get Vector<SignedBeaconBlock> from step
pub fn get_blocks_from_strings(
        block_strs: Vec::<String>) 
    -> Vec<SignedBeaconBlock<MainnetEthSpec>> {
    let block_dir = get_workspace_dir().join("blocks");
    let forkname = ForkName::Capella;
    let spec = forkname.make_genesis_spec(<MainnetEthSpec>::default_spec());
    
    let mut blocks = Vec::new();
    for block in block_strs {
        blocks.push(targets::utils::ssz_decode_file_with(&block_dir.join(format!("{}.ssz_snappy", block)), 
            |bytes| {
                SignedBeaconBlock::from_ssz_bytes(bytes, &spec)
            }).unwrap());

        }
    
    blocks
}

// Get Vector<Attestation> from step
pub fn get_attestations_from_strings(
        attestation_strs: Vec::<String>) 
    -> Vec<Attestation<MainnetEthSpec>> {
    let attestation_dir = get_workspace_dir().join("attestations");
    let forkname = ForkName::Capella;
    let spec = forkname.make_genesis_spec(<MainnetEthSpec>::default_spec());
    
    let mut attestations = Vec::new();
    for attestation in attestation_strs {
        attestations.push(
            targets::utils::ssz_decode_file(
                &attestation_dir.join(format!("{}.ssz_snappy", attestation))
            ).unwrap()
        );
    }
    
    attestations
}

// Get SignedBeaconBlock from block string
pub fn get_block_with_string(block_str: String) -> SignedBeaconBlock<MainnetEthSpec> {
    let block_dir = get_workspace_dir().join("blocks");
    let forkname = ForkName::Capella;
    let spec = forkname.make_genesis_spec(<MainnetEthSpec>::default_spec());
    
    targets::utils::ssz_decode_file_with(&block_dir.join(format!("{}.ssz_snappy", block_str)), 
    |bytes| {
        SignedBeaconBlock::from_ssz_bytes(bytes, &spec)
    }).unwrap()
}

// Get Attestation from attestation string
pub fn get_attestation_with_string(attestation_str: String) -> Attestation<MainnetEthSpec> {
    let attestation_dir = get_workspace_dir().join("attestations");
    let forkname = ForkName::Capella;
    let spec = forkname.make_genesis_spec(<MainnetEthSpec>::default_spec());
    
    targets::utils::ssz_decode_file(
        &attestation_dir.join(format!("{}.ssz_snappy", attestation_str))
    ).unwrap()
}

// Get the leaf blocks from a vector of blocks
pub fn get_leaf_blocks(blocks: Vec<SignedBeaconBlock<MainnetEthSpec>>) -> Vec<SignedBeaconBlock<MainnetEthSpec>> {
    let mut leaf_blocks = Vec::with_capacity(blocks.len());
    let mut temp_blocks = HashMap::with_capacity(blocks.len());
    for block in blocks {
        let block_hash = block.canonical_root();
        let parent_hash = block.message().parent_root();
        temp_blocks.remove(&parent_hash);
        temp_blocks.insert(block_hash, block);
    }
    leaf_blocks.extend(temp_blocks.into_values());
    leaf_blocks
}

// Get a random block from a vector of leaf blocks
pub fn get_random_block(blocks: Vec<SignedBeaconBlock<MainnetEthSpec>>) -> SignedBeaconBlock<MainnetEthSpec> {
    let mut rng = thread_rng();
    let random_index = rng.gen_range(0..blocks.len());
    blocks[random_index].clone()
}

// Get step until target
pub fn get_step_until_target(
        target: String, 
        target_type: String, 
        steps: Vec<Step<String, String, String, String>>) 
    -> Vec<Step<String, String, String, String>> {
    
    let target_name = format!("{}_{}", target_type, target);

    let mut new_steps = Vec::new();
    for step in steps {
        match step {
            Step::Tick { tick } => new_steps.push(Step::Tick { tick }),
            Step::Block { block } => {
                println!("\t\t\t\t[DEBUG] block : {}", block);
                println!("\t\t\t\t[DEBUG] target: {}", target_name);
                if block == target_name {
                    break;
                }
                new_steps.push(Step::Block { block })},
            Step::Attestation { attestation } => {
                println!("\t\t\t\t[DEBUG] attestation : {}", attestation);
                println!("\t\t\t\t[DEBUG] target: {}", target_name);
                if attestation == target_name {
                    break;
                }
                new_steps.push(Step::Attestation { attestation })},
            Step::AttesterSlashing { attester_slashing } => {
                println!("\t\t\t\t[DEBUG] attester_slashing : {}", attester_slashing);
                println!("\t\t\t\t[DEBUG] target: {}", target_name);
                if attester_slashing == target_name {
                    break;
                }
                new_steps.push(Step::AttesterSlashing { attester_slashing })},
            // else do noting
            _ => (),
        }
    }
    new_steps
}


pub fn be_private_key(validator_index: usize) -> [u8; 32] {
    let curve_order: BigUint =
        "52435875175126190479447740508185965837690552500527637822603658699938581184513"
            .parse::<BigUint>()
            .expect("Curve order should be valid");

    let preimage = {
        let mut bytes = [0; 32];
        let index = validator_index.to_le_bytes();
        bytes[0..index.len()].copy_from_slice(&index);
        bytes
    };

    let privkey = BigUint::from_bytes_le(&hash(&preimage)) % curve_order.clone();
    println!("privkey: {}", privkey);
    // let privkey = BigUint::from_bytes_le(&hash(&privkey_.to_bytes_le())) % curve_order.clone();

    let mut bytes = [0; 32];
    let privkey_bytes = privkey.to_bytes_be();
    bytes[32 - privkey_bytes.len()..].copy_from_slice(&privkey_bytes);

    bytes
}

pub fn keypair(validator_index: usize) -> Keypair {
    let sk = SecretKey::deserialize(&be_private_key(validator_index)).unwrap_or_else(|_| {
        panic!(
            "Should build valid private key for validator index {}",
            validator_index
        )
    });

    Keypair::from_components(sk.public_key(), sk)
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub parent_root: Hash256,
    pub root: Hash256,
    pub checkpoint: Checkpoint,
    pub height: u32,
    pub childs: Vec<Hash256>,
    pub vote_count: u32,
    pub attestations: Vec<usize>,
    pub block: SignedBeaconBlock<MainnetEthSpec>,
    pub post_state: BeaconState<MainnetEthSpec>,
}

impl BlockNode {
    pub fn anchor() -> BlockNode {
        let path = get_workspace_dir();
        let forkname = ForkName::Capella;
        let spec = forkname.make_genesis_spec(<MainnetEthSpec>::default_spec());
        let anchor_state: BeaconState<MainnetEthSpec> 
            = targets::utils::ssz_decode_state(&path.join("anchor_state").join("anchor_state.ssz_snappy"), &spec).unwrap();
        let anchor_block = targets::utils::ssz_decode_file_with(&path.join("anchor_block").join("anchor_block.ssz_snappy"), 
            |bytes| {
                BeaconBlock::from_ssz_bytes(bytes, &spec)
            }).unwrap();
        let signed = anchor_block.sign(
            &keypair(0).sk,
            &anchor_state.fork(),
            anchor_state.genesis_validators_root(),
            &spec,);
        let root = signed.canonical_root();
        let parent_root = Hash256::zero();
        let vote_count = 0;
        let attestations = Vec::new();
        let checkpoint = anchor_state.current_justified_checkpoint();
        let node = BlockNode {
            parent_root,
            root,
            checkpoint,
            vote_count,
            block: signed,
            attestations: attestations,
            childs: Vec::new(),
            height: 0,
            post_state: anchor_state,
        };
        node
    }

    pub fn new(
        block_data: SignedBeaconBlock<MainnetEthSpec>,
        post_state: BeaconState<MainnetEthSpec>,
    ) -> BlockNode {
        let parent_root = block_data.message().parent_root();
        let root = block_data.canonical_root();
        let vote_count = 0;
        let checkpoint = post_state.current_justified_checkpoint();
        let node = BlockNode {
            parent_root,
            root,
            checkpoint,
            vote_count,
            block: block_data,
            attestations: Vec::new(),
            childs: Vec::new(),
            height: 0,
            post_state,
        };
        node
    }

    pub fn update_vote_count(
        &mut self, n: u32) {
        self.vote_count += n;;
    }
}

#[derive(Debug, Clone)]
pub struct Chain {
    pub blocks: HashMap<Hash256, BlockNode>,
    pub root: Hash256,
    pub depth: u32,
    pub latest_slot: Slot,
    pub current_justified_checkpoint: Checkpoint,
    pub mutatable_pool: HashMap<Hash256, BlockNode>,
}

impl Chain {
    pub fn new() 
    -> Chain {
        let mut blocks = HashMap::new();
        let node = BlockNode::anchor();
        let root = node.root;
        let depth = 0;
        let latest_slot = Slot::new(0);
        blocks.insert(node.root, node);

        Chain {
            blocks,
            root,
            depth,
            latest_slot,
            current_justified_checkpoint: Checkpoint {
                epoch: Epoch::new(0),
                root: root,
            },
            mutatable_pool: HashMap::new(),
        }
    }

    pub fn insert_block_to_chain(
        &mut self, 
        block_data: SignedBeaconBlock<MainnetEthSpec>,
        post_state: BeaconState<MainnetEthSpec>,
    ) -> Result<(), String> {
        let mut node = BlockNode::new(block_data.clone(), post_state.clone());
        let root = node.root;

        if let Some(parent) = self.blocks.get(&node.parent_root) {
            // update latest slot
            let slot = node.block.message().slot();
            if slot > self.latest_slot {
                self.latest_slot = slot;
            }
            // update height of node
            node.height = parent.height + 1;
            // update child of parent
            let mut parent = parent.clone();
            parent.childs.push(root);
            self.blocks.insert(node.parent_root, parent);
            // insert node
            self.blocks.insert(root, node.clone());
            // update depth of chain
            self.depth = std::cmp::max(self.depth, node.height);
            return Ok(());
        }
        let current_justified = node.post_state.current_justified_checkpoint();
        if self.current_justified_checkpoint.epoch < current_justified.epoch {
            self.current_justified_checkpoint = current_justified;
        }
        // Do we handle fully wrong block that will be early rejected?
        Err("Parent block not found".to_string())
    }

    pub fn set_vote(&mut self, attestations: Vec<Attestation<MainnetEthSpec>>, blocks: Vec<SignedBeaconBlock<MainnetEthSpec>>) {
        for attestation in attestations {
            // default vote (unaggregated = 1), if it has aggregated bits, use it as vote count
            let count = std::cmp::max(1, attestation.aggregation_bits.num_set_bits());
            let attest_target_block = attestation.data.beacon_block_root;
    
            // update vote count of target block node and propagate to parents
            // TODO: until reach checkpoint
            if let Some(target_node) = self.blocks.get_mut(&attest_target_block) {
                target_node.update_vote_count(count.try_into().unwrap());
    
                let mut parent_root = target_node.parent_root;
                while parent_root != attestation.data.source.root {
                    if let Some(parent_node) = self.blocks.get_mut(&parent_root) {
                        parent_node.update_vote_count(count.try_into().unwrap());
                        parent_root = parent_node.parent_root;
                    } else {
                        break;
                    }
                }
            }
        }
        self.set_vote_for_attested_block(blocks);
    }

    pub fn set_vote_for_attested_block(&mut self, blocks: Vec<SignedBeaconBlock<MainnetEthSpec>>) {
        for block in blocks {
            if block.message().body().attestations().is_empty() {
                continue;
            } else {
                let attestations = block.message().body().attestations().clone();
                for attestation in attestations {
                    // default vote (unaggregated = 1), if it has aggregated bits, use it as vote count
                    let count = std::cmp::max(1, attestation.aggregation_bits.num_set_bits());
                    let attest_target_block = attestation.data.beacon_block_root;
            
                    // update vote count of target block node and propagate to parents
                    let mut parent_root = block.parent_root();
                    while parent_root != attestation.data.source.root {
                        if let Some(parent_node) = self.blocks.get_mut(&parent_root) {
                            parent_node.update_vote_count(count.try_into().unwrap());
                            parent_root = parent_node.parent_root;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
    pub fn set_mutatable_pool(&mut self) {
        let next_slot_epoch = (self.latest_slot + 1) / SLOT_PER_EPOCH;
        let allowed_epoch = next_slot_epoch - 1;
        let allowed_epoch_start_slot = allowed_epoch * SLOT_PER_EPOCH;
        for (block_root, block_node) in self.blocks.clone() {
            if block_node.block.slot() >= allowed_epoch_start_slot {
                self.mutatable_pool.insert(block_root, block_node);
            }
        }
        if self.mutatable_pool.contains_key(&self.root) {
            self.mutatable_pool.remove(&self.root);
        }
    }

    pub fn get_canonical_leaf_node(&self) 
    -> Option<BlockNode> {
        let mut node = self.blocks.get(&self.current_justified_checkpoint.root)?.clone();
        loop {
            // Check if the node is a leaf node
            if node.childs.is_empty() {
                println!("[DEBUG] Canonical leaf node: {:?}", node.root);
                return Some(node);
            }
            // Find the child node with the highest vote count
            let mut highest_vote_count = 0;
            let mut highest_vote_child = None;

            for child_root in node.childs.iter() {
                if let Some(child_node) = self.blocks.get(child_root) {
                    if child_node.vote_count > highest_vote_count {
                        highest_vote_count = child_node.vote_count;
                        highest_vote_child = Some(child_node.clone());
                    }
                }
            }
    
            // If there are no children with votes
            if highest_vote_child.is_none() {
                if node.childs.len() == 0 {
                    // if no child, return current node
                    println!("[DEBUG] Canonical leaf node: {:?}", node.root);
                    return Some(node);
                } else {
                    // else follow branch until reach leaf
                    while node.childs.len() > 0 {
                        node = self.blocks.get(node.childs.first().unwrap()).unwrap().clone();
                    }

                    let mut highest_leaf_node: BlockNode = node.clone();
                    let mut highest_height: u32 = 0;

                    for child_root in &node.childs {
                        if let Some(child_node) = self.blocks.get(child_root) {
                            if child_node.height > highest_height {
                                highest_leaf_node = child_node.clone();
                                highest_height = child_node.height;
                            }
                        }
                    }
                    return Some(highest_leaf_node.clone());
                }

            }
            node = highest_vote_child.unwrap();
        }
    }

    pub fn get_leaf_blocks(&self) -> Vec<&BlockNode> {
        self.mutatable_pool.values().filter(|block| block.childs.is_empty()).collect()
    }

    pub fn get_non_leaf_blocks(&self) -> Vec<&BlockNode> {
        let non_leafs: Vec<&BlockNode> = self.mutatable_pool.values().filter(|block| !block.childs.is_empty()).collect();
        if non_leafs.len() == 0 {
            return self.get_leaf_blocks();
        } else {
            return non_leafs;
        }
    }

    pub fn get_highest_leaf_block(&self) 
    -> SignedBeaconBlock<MainnetEthSpec> {
        let mut highest_leaf_block: Option<&BlockNode> = None;
        let mut highest_height = 0;
    
        for block in self.mutatable_pool.values() {
            if block.childs.is_empty() && block.height >= highest_height {
                highest_leaf_block = Some(block);
                highest_height = block.height;
            }
        }
    
        let block = &highest_leaf_block.unwrap().block;
        block.clone()
    }

    pub fn get_shortest_leaf_block(&self)
    -> SignedBeaconBlock<MainnetEthSpec> {
        let mut shortest_leaf_block: Option<&BlockNode> = None;
        let mut shortest_height = std::u32::MAX;
    
        for block in self.mutatable_pool.values() {
            if block.childs.is_empty() && block.height <= shortest_height {
                shortest_leaf_block = Some(block);
                shortest_height = block.height;
            }
        }
    
        let block = &shortest_leaf_block.unwrap().block;
        block.clone()
    }

    pub fn get_branch(&self, leaf_hash: Hash256) -> Option<Vec<BlockNode>> {
        let mut branch = vec![];
        println!("[DEBUG][get_branch] Leaf hash: {:?}", leaf_hash);
        if let Some(leaf_node) = self.blocks.get(&leaf_hash) {
            let mut current_node = leaf_node;
            while current_node.parent_root != Hash256::zero() {
                branch.push(current_node.clone());
                if let Some(parent_node) = self.blocks.get(&current_node.parent_root) {
                    current_node = parent_node;
                } else {
                    // Parent node not found, branch is incomplete
                    return None;
                }
            }
            // Add root node to branch
            // branch.push(current_node.clone());
            // Reverse branch to start from root
            branch.reverse();
            Some(branch)
        } else {
            None
        }
    }

    pub fn get_random_block(&self) -> Option<BlockNode> {
        let keys: Vec<&Hash256> = self.mutatable_pool.keys().collect();
        if keys.is_empty() {
            None
        } else {
            let random_key = keys.choose(&mut rand::thread_rng())?;
            self.blocks.get(random_key).cloned()
        }
    }

    pub fn find_common_ancestor(
        &self, 
        block_node_1: &BlockNode, 
        block_node_2: &BlockNode) 
    -> BlockNode {
        // Get the height of each block node
        let height_1 = block_node_1.height;
        let height_2 = block_node_2.height;
        println!("[DEBUG] height: {:?}, {:?}", height_1, height_2);

        if height_1 == 0 || height_2 == 0 {
            return self.blocks.get(&self.root).unwrap().clone();
        }

        // Find the highest common ancestor
        let mut node_1 = block_node_1;
        let mut node_2 = block_node_2;
        while node_1.height > node_2.height {
            node_1 = self.blocks.get(&node_1.parent_root).unwrap();
        }
        while node_2.height > node_1.height {
            node_2 = self.blocks.get(&node_2.parent_root).unwrap();
        }
        println!("[DEBUG] height: {:?}, {:?}", node_1.height, node_2.height);
        while node_1.parent_root != node_2.parent_root {
            node_1 = self.blocks.get(&node_1.parent_root).unwrap();
            node_2 = self.blocks.get(&node_2.parent_root).unwrap();
        }
        
        // Return the common ancestor
        self.blocks.get(&node_2.parent_root).clone().unwrap().clone()

    }
    
    // get score of a branch (only count votes, do not consider proposer LMD Score boosting)
    pub fn get_branch_score(&self, block_node: &BlockNode, ancestor: &BlockNode) -> u32 {
        let mut score = 0;
        let mut current_node = block_node;
        while current_node.height > 0 {
            score = current_node.vote_count;
            if let Some(parent_node) = self.blocks.get(&current_node.parent_root) {
                if parent_node.root == ancestor.root {
                    break;
                } else {
                    current_node = parent_node;
                }
            } else {
                // Parent node not found
                // MAybe branch is incomplete (error), or test case only have a root node
                return 0;
            }
        }
        score
    }

    // get post_state after block processing without testing harness
    pub fn get_poststate(
        &self,
        block: SignedBeaconBlock<MainnetEthSpec>,)
    -> BeaconState<MainnetEthSpec> {
        let forkname = ForkName::Capella;
        let spec = forkname.make_genesis_spec(<MainnetEthSpec>::default_spec());
        let mut state = self.blocks.get(&block.message().parent_root()).unwrap().post_state.clone();
        println!("[DEBUG]block.parent_root: {:?}", block.message().parent_root());
        // *state.slot_mut() = block.slot();
        while state.slot() < block.slot() {
            // TODO(gnattishness) handle option
            // requires implementation of an error trait that I can specify as the
            // return type
            per_slot_processing(&mut state, None, &spec).unwrap();
        }
        state.build_all_caches(&spec).unwrap();
        state.build_committee_cache(RelativeEpoch::Current, &spec).unwrap();
        println!("[DEBUG] pre state slot: {:?}, block slot: {:?}", state.slot(), block.slot());
        let mut ctxt = ConsensusContext::new(state.slot());
        per_block_processing(
            &mut state,
            &block,
            BlockSignatureStrategy::VerifyIndividual,
            VerifyBlockRoot::True,
            &mut ctxt,
            &spec,
        ).unwrap();
        assert_eq!(state.tree_hash_root(), block.state_root());
        state
    }

}
