use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum FuzzingTargets {
    // Lighthouse
    Block,
    ForkChoice,
}

impl FuzzingTargets {
    pub fn name(&self) -> String {
        match &self {
            // Lighthouse
            FuzzingTargets::Block => "block",
            FuzzingTargets::ForkChoice => "fc"
        }
        .to_string()
    }

    pub fn corpora(&self) -> String {
        match &self {
            // Lighthouse
            FuzzingTargets::Block => "block_corpus",
            FuzzingTargets::ForkChoice => "fc_corpus",
        }
        .to_string()
    }
}

pub fn get_targets() -> Vec<String> {
    FuzzingTargets::iter().map(|x| x.name()).collect()
}