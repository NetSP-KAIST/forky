use state_processing::{
    per_block_processing, BlockSignatureStrategy, BlockProcessingError, ConsensusContext, VerifyBlockRoot,
};

use types::{BeaconState, EthSpec, MainnetEthSpec, RelativeEpoch, SignedBeaconBlock};

    /// Run `process_block`
    /// TODO N convert to a library in lighthouse that we import
    /// Most of this is copied from lighthouse/ef_tests/src/cases/sanity_blocks.rs,
    /// as lighthouse doesn't directly implement the state_transition function.
pub fn state_transition(beaconstate: BeaconState<MainnetEthSpec>,
    block: SignedBeaconBlock<MainnetEthSpec>,
    validate_state_root: bool)
        -> Result<(), BlockProcessingError> {

    let spec = &MainnetEthSpec::default_spec();
    let mut state = beaconstate; // No need to clone here, but means state_transition can only be called once?

    // TODO(gnattishness) any reason why we would want to unwrap and panic here vs returning an error?
    state.build_all_caches(spec)?;

    let result = {
        /* not good for fuzzing (in some spectests cases, this can loop for 65k iteration )
        while state.slot < block.slot() {
            // TODO(gnattishness) handle option
            // requires implementation of an error trait that I can specify as the
            // return type
            per_slot_processing(&mut state, None, spec).unwrap();
        }
        */
        state
            .build_committee_cache(RelativeEpoch::Current, spec)
            .unwrap();

        let mut ctxt = ConsensusContext::new(state.slot());
        per_block_processing(
            &mut state,
            &block,
            BlockSignatureStrategy::VerifyIndividual,
            VerifyBlockRoot::True,
            &mut ctxt,
            spec,
        )?;
        state
    };

    if !validate_state_root || block.state_root() == result.canonical_root() {
        Ok(())
    } else {
        Err(BlockProcessingError::StateRootMismatch)
    }
}
