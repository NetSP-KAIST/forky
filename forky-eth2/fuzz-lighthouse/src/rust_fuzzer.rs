use failure::{Error, ResultExt};
use std::env;
use std::path::PathBuf;
use std::process::Command;

use crate::env::{corpora_dir, state_dir};
pub use crate::fuzzer_params::{FuzzerConfig, FuzzerQuit};
use crate::fuzzing_targets::FuzzingTargets;

/***********************************************
name: libfuzzer/cargo-fuzz
github: https://github.com/rust-fuzz/cargo-fuzz
***********************************************/

pub struct FuzzerLibfuzzer {
    /// Fuzzer name.
    pub name: String,
    /// Source code / template dir
    pub dir: PathBuf,
    /// Workspace dir
    pub work_dir: PathBuf,
    /// fuzzing config
    pub config: FuzzerConfig,
}

impl FuzzerLibfuzzer {
    /// Check if `cargo fuzz` is installed
    pub fn is_available() -> Result<(), Error> {
        println!("[fuzz-lighthouse] Testing FuzzerLibfuzzer is available");
        let fuzzer_output = Command::new("cargo")
            .arg("fuzz")
            .arg("--version")
            .output()?;
        if !fuzzer_output.status.success() {
            bail!("cargo-fuzz not available, install with `cargo install --force cargo-fuzz`");
        }
        Ok(())
    }

    /// Create a new FuzzerLibfuzzer
    pub fn new(config: FuzzerConfig) -> Result<FuzzerLibfuzzer, Error> {
        // println!("[DEBUG] CALL: new");
        // Test if fuzzer engine installed
        FuzzerLibfuzzer::is_available()?;

        let cwd = env::current_dir().context("error getting current directory")?;
        let fuzzer = FuzzerLibfuzzer {
            name: "Libfuzzer".to_string(),
            dir: cwd.join("fuzzer").join("rust-libfuzzer"),
            work_dir: cwd.join("workspace").join("fuzz"),
            config,
        };
        // println!("[DEBUG] END: new");
        Ok(fuzzer)
    }

    pub fn run(&self, target: FuzzingTargets) -> Result<(), Error> {
        // println!("[DEBUG] CALL: run");


        // prepare_targets_workspace()?;
        // create afl folder inside workspace/
        // self.prepare_fuzzer_workspace()?;

        /*
                let fuzz_dir = self.work_dir.join("fuzz");
                fs::create_dir_all(&fuzz_dir)
                    .context(format!("unable to create {} dir", fuzz_dir.display()))?;

                let target_dir = fuzz_dir.join("fuzz_targets");

                let _ = fs::remove_dir_all(&target_dir)
                    .context(format!("error removing {}", target_dir.display()));
                fs::create_dir_all(&target_dir)
                    .context(format!("unable to create {} dir", target_dir.display()))?;

                fs::create_dir_all(&fuzz_dir)
                    .context(format!("unable to create {} dir", fuzz_dir.display()))?;
                //println!("{:?}", fuzz_dir);

                fs::copy(
                    self.dir.join("fuzz").join("Cargo.toml"),
                    fuzz_dir.join("Cargo.toml"),
                )?;

                // Add all targets to libfuzzer
                for target in Targets::iter().filter(|x| x.language() == "rust") {
                    write_libfuzzer_target(&self.work_dir, target)?;
                }
        */
        let fuzz_dir = env::current_dir()?.join("fuzz");
        let corpus_dir = corpora_dir()?.join(target.corpora());
        // println!("[DEBUG] END: corpora_dir");
        // sanitizers
        let rust_args = format!(
            "{} \
            {}",
            if let Some(san) = self.config.sanitizer {
                format!("-Z sanitizer={}", san.name())
            } else {
                "".into()
            },
            env::var("RUSTFLAGS").unwrap_or_default()
        );

        // create arguments
        // corpora dir
        // max_time if provided (i.e. continuously fuzzing)
        let mut args: Vec<String> = Vec::new();
        args.push(format!("{}", &corpus_dir.display()));
        if let Some(timeout) = self.config.timeout {
            args.push("--".to_string());
            args.push(format!("-max_total_time={}", timeout));
        };
        // threading
        if let Some(thread) = self.config.thread {
            args.push(format!("-workers={}", thread));
            args.push(format!("-jobs={}", thread));
        };
        // handle seed option
        if let Some(seed) = self.config.seed {
            args.push(format!("-seed={}", seed));
        };

        // Launch the fuzzer using cargo
        // println!("[DEBUG] START: Launch the fuzzer");
        println!("{}", env::current_dir()?.display());
        let fuzzer_bin = Command::new("cargo")
            .args(&["+nightly", "fuzz", "run", &target.name()])
            .args(&args)
            .env(
                "ETH2FUZZ_BEACONSTATE",
                format!("{}", state_dir()?.display()),
            )
            .env("RUSTFLAGS", &rust_args)
            .current_dir(&fuzz_dir)
            .spawn()
            .context(format!(
                "error starting {:?} to run {}",
                self.name,
                target.name()
            ))?
            .wait()
            .context(format!(
                "error while waiting for {:?} running {}",
                self.name,
                target.name()
            ))?;

        if !fuzzer_bin.success() {
            return Err(FuzzerQuit.into());
        }
        Ok(())
    }
}
