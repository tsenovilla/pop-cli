// SPDX-License-Identifier: GPL-3.0

//! Integration tests for chain-related functionality.

#![cfg(all(feature = "chain", feature = "integration-tests"))]

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
	let working_dir = temp_dir.join("test_parachain");
	if !working_dir.exists() {
		let mut command = pop(
			temp_dir,
			[
				"new",
				"chain",
				"test_parachain",
				"--symbol",
				"POP",
				"--decimals",
				"6",
				"--endowment",
				"1u64 << 60",
				"--verify",
				"--with-frontend=create-dot-app",
				"--package-manager",
				"npm",
			],
		);
		assert!(command.spawn()?.wait().await?.success());
		assert!(working_dir.exists());
		assert!(working_dir.join("frontend").exists());
	}


	// pop build spec --output ./target/pop/test-spec.json --para-id 2222 --type development --relay
	// paseo-local --protocol-id pop-protocol --chain local --deterministic=false
	// --default-bootnode=false
	let mut command = pop(
		&working_dir,
		[
			"build",
			"spec",
			"--output",
			"./target/pop/test-spec.json",
			"--id",
			"test-chain",
			"--para-id",
			"2222",
			"--type",
			"development",
			"--chain",
			"local_testnet",
			"--relay",
			"paseo-local",
			"--profile",
			"release",
			"--raw",
			"--genesis-state=true",
			"--genesis-code=true",
			"--protocol-id",
			"pop-protocol",
			"--deterministic=false",
			"--default-bootnode=false",
			"--skip-build",
		],
	);
	assert!(command.spawn()?.wait().await?.success());

	// Assert build files have been generated
	assert!(working_dir.join("target").exists());
	assert!(working_dir.join("target/pop/test-spec.json").exists());
	assert!(working_dir.join("target/pop/test-spec-raw.json").exists());
	assert!(working_dir.join("target/pop/genesis-code.wasm").exists());
	assert!(working_dir.join("target/pop/genesis-state").exists());

	let chain_spec_path = working_dir.join("target/pop/test-spec.json");
	let content = fs::read_to_string(&chain_spec_path).expect("Could not read file");
	// Assert custom values have been set properly
	assert!(content.contains("\"para_id\": 2222"));
	// assert!(content.contains("\"tokenDecimals\": 6"));
	// assert!(content.contains("\"tokenSymbol\": \"POP\""));
	assert!(content.contains("\"relay_chain\": \"paseo-local\""));
	assert!(content.contains("\"protocolId\": \"pop-protocol\""));
	assert!(content.contains("\"id\": \"test-chain\""));


	// Overwrite the config file to manually set the port to test pop call parachain.
	let network_toml_path = working_dir.join("network.toml");
	fs::create_dir_all(&working_dir)?;
	let random_port = find_free_port(None);
	let localhost_url = format!("ws://127.0.0.1:{}", random_port);
	fs::write(
		&network_toml_path,
		format!(
			r#"[relaychain]
chain = "paseo-local"

[[relaychain.nodes]]
name = "alice"
validator = true

[[relaychain.nodes]]
name = "bob"
validator = true

[[parachains]]
id = 2000
default_command = "polkadot-omni-node"
chain_spec_path = "{}"

[[parachains.collators]]
name = "collator-01"
rpc_port = {random_port}
"#,
			chain_spec_path.as_os_str().to_str().unwrap(),
		),
	)?;

	// `pop up network ./network.toml --skip-confirm`
	let mut command = pop(
		&working_dir,
		["up", "network", "./network.toml", "-r", "stable2506-2", "--verbose", "--skip-confirm"],
	);
	assert!(command.spawn()?.wait().await?.success());

	// Wait for the networks to initialize. Increased timeout to accommodate CI environment delays.
	let wait = Duration::from_secs(300);
	println!("waiting for {wait:?} for network to initialize...");
	tokio::time::sleep(wait).await;

	// `pop call chain --pallet System --function remark --args "0x11" --url
	// ws://127.0.0.1:random_port --suri //Alice --skip-confirm`
	let mut command = pop(
		&working_dir,
		[
			"call",
			"chain",
			"--pallet",
			"System",
			"--function",
			"remark",
			"--args",
			"0x11",
			"--url",
			&localhost_url,
			"--suri",
			"//Alice",
			"--skip-confirm",
		],
	);
	assert!(command.spawn()?.wait().await?.success());

	// `pop call chain --pallet System --function Account --args
	// "15oF4uVJwmo4TdGW7VfQxNLavjCXviqxT9S1MgbjMNHr6Sp5" --url ws://127.0.0.1:random_port
	// --skip-confirm`
	let mut command = pop(
		&working_dir,
		[
			"call",
			"chain",
			"--pallet",
			"System",
			"--function",
			"Account",
			"--args",
			"15oF4uVJwmo4TdGW7VfQxNLavjCXviqxT9S1MgbjMNHr6Sp5",
			"--url",
			&localhost_url,
			"--skip-confirm",
		],
	);
	assert!(command.spawn()?.wait().await?.success());

	// `pop call chain --pallet System --function Account --args
	// "15oF4uVJwmo4TdGW7VfQxNLavjCXviqxT9S1MgbjMNHr6Sp5" --url ws://127.0.0.1:random_port`
	let mut command = pop(
		&working_dir,
		[
			"call",
			"chain",
			"--pallet",
			"System",
			"--function",
			"Ss58Prefix",
			"--url",
			&localhost_url,
			"--skip-confirm",
		],
	);
	assert!(command.spawn()?.wait().await?.success());

	// pop call chain --call 0x00000411 --url ws://127.0.0.1:random_port --suri //Alice
	// --skip-confirm
	let mut command = pop(
		&working_dir,
		[
			"call",
			"chain",
			"--call",
			"0x00000411",
			"--url",
			&localhost_url,
			"--suri",
			"//Alice",
			"--skip-confirm",
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
