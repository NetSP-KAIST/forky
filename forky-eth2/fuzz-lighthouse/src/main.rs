extern crate regex;
extern crate structopt;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;

// Strum contains all the trait definitions
extern crate strum;
extern crate strum_macros;

use crate::strum::IntoEnumIterator;
use failure::Error;

use structopt::StructOpt;


// load fuzzers
mod env;
// load utily methods
mod utils;
// load rust fuzzers
mod rust_fuzzer;
// load fuzzer parameters
mod fuzzer_params;
// load fuzzing targets
mod fuzzing_targets;

/// Run eth2fuzz fuzzing targets
#[derive(StructOpt, Debug)]
enum Cli {
    /// Run all fuzz targets
    #[structopt(name = "continuously")]
    Continuous {
        /// Only run target containing this eth2 clients name (e.g. lighthouse)
        #[structopt(short = "q", long = "filter")]
        filter: String,
        /// Set timeout per target (in seconds)
        #[structopt(short = "t", long = "timeout", default_value = "1800")]
        timeout: i32,
        /// Set number of thread
        #[structopt(short = "n", long = "thread")]
        thread: Option<i32>,
        /// Set seed value
        #[structopt(short = "s", long = "seed")]
        seed: Option<i32>,
        /// Set a compilation sanitizer (advanced)
        #[structopt(
            long = "sanitizer",
            possible_values = &fuzzer_params::Sanitizer::variants(),
            case_insensitive = true
        )]
        sanitizer: Option<fuzzer_params::Sanitizer>,
        // Run until the end of time (or Ctrl+C)
        #[structopt(short = "i", long = "infinite")]
        infinite: bool,
    },
    /// Run one target with specific fuzzer
    #[structopt(name = "target")]
    Run {
        /// Which target to run
        target: String,
        /// Set timeout (in seconds)
        #[structopt(short = "t", long = "timeout")]
        timeout: Option<i32>,
        /// Set number of thread (only for hfuzz)
        #[structopt(short = "n", long = "thread")]
        thread: Option<i32>,
        /// Set seed value
        #[structopt(short = "s", long = "seed")]
        seed: Option<i32>,
        /// Set a compilation sanitizer (advanced)
        #[structopt(
            long = "sanitizer",
            possible_values = &fuzzer_params::Sanitizer::variants(),
            case_insensitive = true
        )]
        sanitizer: Option<fuzzer_params::Sanitizer>,
    },
    /// List all available targets
    #[structopt(name = "list")]
    ListTargets,
}

/// Parsing of CLI arguments
fn run() -> Result<(), Error> {
    use Cli::*;
    let cli = Cli::from_args();

    match cli {
        // list all targets
        ListTargets => {
            list_targets()?;
        }
        // Fuzz one target
        Run {
            target,
            timeout,
            thread,
            seed,
            sanitizer,
        } => {
            let config = rust_fuzzer::FuzzerConfig {
                timeout,
                thread,
                sanitizer,
                seed,
            };
            run_target(target, config)?;
        }
        // Fuzz multiple targets
        Continuous {
            filter,
            timeout,
            thread,
            seed,
            sanitizer,
            infinite,
        } => {
            let config = rust_fuzzer::FuzzerConfig {
                timeout: Some(timeout),
                thread,
                sanitizer,
                seed,
            };
            run_continuously(Some(filter), config, infinite)?;
        }
    }
    Ok(())
}

/// List all targets available
fn list_targets() -> Result<(), Error> {
    let list_targets = fuzzing_targets::get_targets();
    for target in list_targets {
        println!("{}", target);
    }
    Ok(())
}


/// Run fuzzing on only one target
fn run_target(
    target: String,
    config: rust_fuzzer::FuzzerConfig,
) -> Result<(), Error> {
    // println!("[DEBUG] CALL: run_target");
    let target = match fuzzing_targets::FuzzingTargets::iter().find(|x| x.name() == target) {
        None => bail!(
            "Don't know target `{}`. {}",
            target,
            if let Some(alt) = utils::did_you_mean(&target, &fuzzing_targets::get_targets()) {
                format!("Did you mean `{}`?", alt)
            } else {
                "".into()
            }
        ),
        Some(t) => t,
    };

    let lfuzz = rust_fuzzer::FuzzerLibfuzzer::new(config)?;
    lfuzz.run(target)?;
        
    Ok(())
}

/// Run fuzzing on multiple target matching the filter option
fn run_continuously(
    filter: Option<String>,
    config: rust_fuzzer::FuzzerConfig,
    infinite: bool,
) -> Result<(), Error> {
    let run = |target: &str| -> Result<(), Error> {
        let target = match fuzzing_targets::FuzzingTargets::iter().find(|x| x.name() == target) {
            None => bail!(
                "Don't know target `{}`. {}",
                target,
                if let Some(alt) = utils::did_you_mean(&target, &fuzzing_targets::get_targets()) {
                    format!("Did you mean `{}`?", alt)
                } else {
                    "".into()
                }
            ),
            Some(t) => t,
        };

        let lfuzz = rust_fuzzer::FuzzerLibfuzzer::new(config)?;
        lfuzz.run(target)?;

        Ok(())
    };

    let targets = fuzzing_targets::get_targets();
    let targets = targets
        .iter()
        .filter(|x| filter.as_ref().map(|f| x.contains(f)).unwrap_or(true));

    'cycle: loop {
        'targets_pass: for target in targets.clone() {
            if let Err(e) = run(target) {
                match e.downcast::<rust_fuzzer::FuzzerQuit>() {
                    Ok(_) => {
                        println!("Fuzzer failed so we'll continue with the next one");
                        continue 'targets_pass;
                    }
                    Err(other_error) => return Err(other_error),
                }
            }
        }

        if !infinite {
            break 'cycle;
        }
    }
    Ok(())
}


/// Main function catching errors
fn main() {
    println!("[+] Fuzzing lighthouse");
    if let Err(e) = run() {
        eprintln!("[-] {}", e);
        for cause in e.iter_chain().skip(1) {
            eprintln!("[-] caused by: {}", cause);
        }
        ::std::process::exit(1);
    }
}
