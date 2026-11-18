//! Dependency-rule enforcement (SPEC.md 4, PLAN.md M0).
//!
//! Verifies at test time that architectural layering rules hold:
//! - gameplay/server crates never depend on GPU/windowing/audio/UI crates,
//! - gameplay crates never depend on `rand` (GameRandom/CosmeticRandom split),
//! - `client` never depends on `sim`/`server`,
//! - the dedicated server binary has no client/GPU deps.
//!
//! Uses `cargo metadata --no-deps` so it stays fast and offline.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// Crates that are GPU/windowing/audio/UI-ish (also matched as external deps).
const CLIENT_SIDE_CRATES: &[&str] = &[
    "mclre-render",
    "mclre-ui",
    "mclre-client",
    "mclre-audio",
    "mclre-input",
    "mclre-assets",
];
/// External crates that must never appear in gameplay/server dependency trees.
const CLIENT_SIDE_EXTERNAL: &[&str] = &["wgpu", "winit", "kira", "rodio", "gilrs"];

/// Server/gameplay crates: no GPU/windowing/audio/UI deps.
const GAMEPLAY_CRATES: &[&str] = &[
    "mclre-core",
    "mclre-profiles",
    "mclre-world",
    "mclre-worldgen",
    "mclre-sim",
    "mclre-protocol",
    "mclre-net",
    "mclre-server",
    "mclre-content-vanilla",
    "mclre-identity",
];

/// Pure simulation/world crates: additionally must not depend on `rand`.
const PURE_GAMEPLAY_CRATES: &[&str] = &[
    "mclre-core",
    "mclre-world",
    "mclre-worldgen",
    "mclre-sim",
    "mclre-content-vanilla",
];

fn workspace_metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("devtools lives at crates/tools/devtools")
        .join("Cargo.toml");
    let output = Command::new(&cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(&manifest)
        .output()
        .expect("failed to run `cargo metadata`");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata produced invalid JSON")
}

/// Returns (normal dependency package names, external dependency crate names)
/// for each workspace member, ignoring dev/build dependencies.
fn workspace_deps() -> BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> {
    let meta = workspace_metadata();
    let mut out = BTreeMap::new();
    for pkg in meta["packages"].as_array().expect("packages array") {
        let name = pkg["name"].as_str().expect("package name").to_string();
        let mut internal = BTreeSet::new();
        let mut external = BTreeSet::new();
        for dep in pkg["dependencies"].as_array().expect("deps array") {
            let kind = dep["kind"].as_str().unwrap_or("normal");
            if kind != "normal" {
                continue;
            }
            let dep_name = dep["name"].as_str().expect("dep name").to_string();
            if dep["path"].is_null() {
                external.insert(dep_name);
            } else {
                internal.insert(dep_name);
            }
        }
        out.insert(name, (internal, external));
    }
    out
}

fn assert_no_deps(
    subject: &str,
    deps: &BTreeSet<String>,
    forbidden: &[&str],
    rule: &str,
    violations: &mut Vec<String>,
) {
    for f in forbidden {
        if deps.contains(*f) {
            violations.push(format!(
                "{subject} depends on `{f}` — violates rule: {rule}"
            ));
        }
    }
}

#[test]
fn dependency_rules_hold() {
    let deps = workspace_deps();
    let mut violations = Vec::new();

    for &crate_name in GAMEPLAY_CRATES {
        let (internal, external) = deps
            .get(crate_name)
            .unwrap_or_else(|| panic!("workspace member {crate_name} missing from metadata"));
        assert_no_deps(
            crate_name,
            internal,
            CLIENT_SIDE_CRATES,
            "gameplay/server crates must not depend on GPU/windowing/audio/UI crates",
            &mut violations,
        );
        assert_no_deps(
            crate_name,
            external,
            CLIENT_SIDE_EXTERNAL,
            "gameplay/server crates must not depend on GPU/windowing/audio/UI crates",
            &mut violations,
        );
    }

    for &crate_name in PURE_GAMEPLAY_CRATES {
        let (internal, external) = deps
            .get(crate_name)
            .unwrap_or_else(|| panic!("workspace member {crate_name} missing from metadata"));
        assert_no_deps(
            crate_name,
            internal,
            &["mclre-cosmetic-rand"],
            "pure gameplay crates use GameRandom, not a rand-based crate",
            &mut violations,
        );
        assert_no_deps(
            crate_name,
            external,
            &["rand"],
            "pure gameplay crates must not depend on `rand` (GameRandom/CosmeticRandom split)",
            &mut violations,
        );
    }

    if let Some((internal, _)) = deps.get("mclre-client") {
        assert_no_deps(
            "mclre-client",
            internal,
            &["mclre-sim", "mclre-server"],
            "client must not depend on sim/server internals",
            &mut violations,
        );
    }

    if let Some((internal, external)) = deps.get("mclre-server-bin") {
        assert_no_deps(
            "mclre-server-bin",
            internal,
            CLIENT_SIDE_CRATES,
            "dedicated server must not depend on client-side crates",
            &mut violations,
        );
        assert_no_deps(
            "mclre-server-bin",
            external,
            CLIENT_SIDE_EXTERNAL,
            "dedicated server must not depend on GPU/windowing/audio crates",
            &mut violations,
        );
    }

    assert!(
        violations.is_empty(),
        "dependency-rule violations ({}):\n{}",
        violations.len(),
        violations.join("\n")
    );
}

/// Self-test of the rule engine: injecting a fake violation must be detected.
#[test]
fn rule_engine_detects_violation() {
    let deps: BTreeSet<String> = ["wgpu".to_string(), "mclre-core".to_string()].into();
    let mut violations = Vec::new();
    assert_no_deps(
        "fake-crate",
        &deps,
        CLIENT_SIDE_EXTERNAL,
        "test rule",
        &mut violations,
    );
    assert_eq!(
        violations.len(),
        1,
        "expected the fake wgpu dep to be flagged"
    );
}
