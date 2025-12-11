// SPDX-License-Identifier: GPL-3.0

//! Integration tests for chain-related functionality.

#![cfg(all(feature = "chain", feature = "integration-tests"))]
#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Result;
use pop_chains::{
	ChainTemplate,
	up::{ArchiveType, Source::GitHub, SourcedArchive},
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
				"paseo",
				"--parachain",
				"passet-hub",
			],
		);
		assert!(command.spawn()?.wait().await?.success());


	Ok(())
}

// Function that mocks the build process generating the target dir and release.
fn mock_build_process(temp_dir: &Path) -> Result<()> {
	// Create a target directory
	let target_dir = temp_dir.join("target");
	fs::create_dir_all(target_dir.join("release/wbuild/parachain-template-runtime"))?;
	// Create a release file
	fs::File::create(
		target_dir
			.join("release/wbuild/parachain-template-runtime/parachain_template_runtime.wasm"),
	)?;
	Ok(())
}

/// Fetch binary from GitHub releases
async fn fetch_runtime(cache: &Path) -> Result<String> {
	let name = "parachain_template_runtime.wasm";
	let contents = ["parachain_template_runtime.wasm"];
	let binary = SourcedArchive::Source {
		name: name.to_string(),
		source: GitHub(ReleaseArchive {
			owner: "r0gue-io".into(),
			repository: "base-parachain".into(),
			tag: None,
			tag_pattern: Some("polkadot-{version}".into()),
			prerelease: false,
			version_comparator: sort_by_latest_semantic_version,
			fallback: "stable2503".to_string(),
			archive: "parachain-template-runtime.tar.gz".to_string(),
			contents: contents
				.into_iter()
				.map(|b| ArchiveFileSpec::new(b.into(), None, true))
				.collect(),
			latest: None,
		})
		.into(),
		cache: cache.to_path_buf(),
		archive_type: ArchiveType::Binary,
	};
	binary.source(true, &(), true).await?;
	Ok(name.to_string())
}

// Replace the binary fetched with the mocked binary
fn replace_mock_with_runtime(temp_dir: &Path, runtime_name: String) -> Result<PathBuf> {
	let runtime_path = temp_dir.join(temp_dir.join(runtime_name));
	let content = fs::read(&runtime_path)?;
	write(
		temp_dir.join(
			"target/release/wbuild/parachain-template-runtime/parachain_template_runtime.wasm",
		),
		content,
	)?;
	Ok(runtime_path)
}

fn get_mock_runtime_path() -> PathBuf {
	let binary_path = "../../tests/runtimes/base_parachain_benchmark.wasm";
	std::env::current_dir().unwrap().join(binary_path).canonicalize().unwrap()
}
