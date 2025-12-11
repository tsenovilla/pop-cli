// SPDX-License-Identifier: GPL-3.0

//! Integration tests for chain-related functionality.

#![cfg(all(feature = "chain", feature = "integration-tests"))]
#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Result;
use pop_chains::{
	ChainTemplate,
	up::{Source::GitHub},
};
use pop_common::{
	find_free_port,
	polkadot_sdk::sort_by_latest_semantic_version,
	pop,
	sourcing::{ArchiveFileSpec, GitHub::ReleaseArchive},
	templates::Template,
};
use std::{
	fs,
	fs::write,
	path::{Path, PathBuf},
	time::Duration,
};
use strum::VariantArray;
use tempfile::tempdir;
use tokio::process::Child;


/// Test the parachain lifecycle: new, build, up, call.
#[tokio::test]
async fn parachain_lifecycle() -> Result<()> {
	// For testing locally: set to `true`
	const LOCAL_TESTING: bool = false;

	let temp = tempfile::tempdir()?;
	let temp_dir = match LOCAL_TESTING {
		true => Path::new("./"),
		false => temp.path(),
	};

	// pop new chain test_parachain --verify (default)
		let mut command = pop(
			temp_dir,
			[
				"up",
                "paseo"
			],
		);
		assert!(command.spawn()?.wait().await?.success());

	Ok(())
}