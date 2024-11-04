use failure::{Error, ResultExt};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn root_dir() -> Result<PathBuf, Error> {
    // println!("[DEBUG] CALL: root_dir");
    let p = env::var("CARGO_MANIFEST_DIR")
        .map(From::from)
        .or_else(|_| env::current_dir())?;
    Ok(p)
}

pub fn workspace_dir() -> Result<PathBuf, Error> {
    // println!("[DEBUG] CALL: workspace_dir");
    let p = root_dir()?.join("workspace");
    // println!("[DEBUG] workspace_dir: {:?}", p);
    //fs::create_dir_all(&p).context("unable to create workspace dir".to_string())?;
    Ok(p)
}

pub fn corpora_dir() -> Result<PathBuf, Error> {
    // println!("[DEBUG] CALL: corpora_dir");
    let p = workspace_dir()?.join("corpus");
    // println!("[DEBUG] corpora_dir: {:?}", p);
    Ok(p)
}

pub fn state_dir() -> Result<PathBuf, Error> {
    // println!("[DEBUG] CALL: state_dir");
    let seed_dir = corpora_dir()?.join("beaconstate");
    // println!("[DEBUG] state_dir: {:?}", seed_dir);
    fs::create_dir_all(&seed_dir)
        .context("unable to create corpus/beaconstate dir".to_string())?;
    Ok(seed_dir)
}
