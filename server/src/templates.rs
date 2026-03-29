//! Project templates for the playground mini-IDE.

use std::path::Path;

pub fn populate_template(name: &str, workspace: &Path) -> Result<(), String> {
    let files: &[(&str, &str)] = match name {
        "hello-world" => &HELLO_WORLD,
        "merkle-proof" => &MERKLE_PROOF,
        "voting-circuit" => &VOTING_CIRCUIT,
        _ => return Err(format!("unknown template: {name}")),
    };

    for (path, content) in files {
        let full = workspace.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        std::fs::write(&full, content).map_err(|e| format!("write {path}: {e}"))?;
    }

    Ok(())
}

const HELLO_WORLD: [(&str, &str); 3] = [
    (
        "achronyme.toml",
        r#"[project]
name = "hello-world"
version = "0.1.0"
entry = "src/main.ach"
"#,
    ),
    (
        "src/main.ach",
        r#"// Hello World — Multi-file Achronyme project
import "./utils.ach" as utils

let name = "Achronyme"
print("Hello from " + name + "!")

// Using imported functions
print("2 + 3 = " + utils.add(2, 3).to_string())
print("4 * 5 = " + utils.multiply(4, 5).to_string())

// Arrays
let primes = [2, 3, 5, 7, 11]
print("Primes: " + primes.to_string())
print("Sum: " + primes.reduce(0, fn(acc, x) { acc + x }).to_string())
"#,
    ),
    (
        "src/utils.ach",
        r#"// Utility functions
export fn add(a, b) {
    return a + b
}

export fn multiply(a, b) {
    return a * b
}

export fn square(x) {
    return x * x
}
"#,
    ),
];

const MERKLE_PROOF: [(&str, &str); 3] = [
    (
        "achronyme.toml",
        r#"[project]
name = "merkle-proof"
version = "0.1.0"
entry = "src/main.ach"
"#,
    ),
    (
        "src/main.ach",
        r#"// Merkle Proof of Membership
// Proves you know a secret in a Merkle tree without revealing it
import "./tree.ach" as tree

// Four members with secret keys
let secrets = [0p101, 0p202, 0p303, 0p404]

// Build the Merkle tree
let leaves = tree.commit_all(secrets)
let root = tree.build_root(leaves)

print("=== Merkle Proof of Membership ===")
print("Leaves: " + leaves.to_string())
print("Root:   " + root.to_string())

// Prove membership of secrets[0] without revealing it
let my_leaf = poseidon(secrets[0], 0p0)
let sibling = leaves[1]
let uncle = poseidon(leaves[2], leaves[3])

prove membership(root: Public) {
    // Recompute leaf from secret
    let leaf = poseidon(secrets[0], 0p0)
    // Walk up the tree
    let node = poseidon(leaf, sibling)
    let computed_root = poseidon(node, uncle)
    assert_eq(computed_root, root, "invalid merkle path")
}

print("")
print("Membership proven!")
"#,
    ),
    (
        "src/tree.ach",
        r#"// Merkle tree utilities

export fn commit(secret) {
    return poseidon(secret, 0p0)
}

export fn commit_all(secrets) {
    mut result = []
    for s in secrets {
        result.push(poseidon(s, 0p0))
    }
    return result
}

export fn build_root(leaves) {
    // Binary tree: 4 leaves -> 2 nodes -> 1 root
    let n0 = poseidon(leaves[0], leaves[1])
    let n1 = poseidon(leaves[2], leaves[3])
    return poseidon(n0, n1)
}
"#,
    ),
];

const VOTING_CIRCUIT: [(&str, &str); 3] = [
    (
        "achronyme.toml",
        r#"[project]
name = "voting-circuit"
version = "0.1.0"
entry = "src/main.ach"
"#,
    ),
    (
        "src/main.ach",
        r#"// Anonymous Voting with Zero-Knowledge Proofs
// Each voter can prove they're registered without revealing identity
import "./ballot.ach" as ballot

// Register voters (each has a secret key)
let voter_keys = [0p1001, 0p1002, 0p1003, 0p1004]
let commitments = ballot.register_voters(voter_keys)
let registry = ballot.build_registry(commitments)

print("=== Anonymous Voting ===")
print("Registry root: " + registry.to_string())
print("Registered voters: " + voter_keys.len().to_string())
print("")

// Voter 0 casts a vote for candidate 1
let vote = 0p1
let nullifier = poseidon(voter_keys[0], vote)
print("Casting anonymous vote...")
print("Nullifier: " + nullifier.to_string())

// Build Merkle path for voter 0
let sibling = commitments[1]
let uncle = poseidon(commitments[2], commitments[3])

prove cast_vote(registry: Public, nullifier: Public, vote: Public) {
    // Prove: I know a secret key that's in the registry
    let my_commit = poseidon(voter_keys[0], 0p0)
    let node = poseidon(my_commit, sibling)
    let root = poseidon(node, uncle)
    assert_eq(root, registry, "not a registered voter")

    // Prove: the nullifier matches my key + vote
    assert_eq(poseidon(voter_keys[0], vote), nullifier, "nullifier mismatch")
}

print("")
print("Vote cast and verified!")
print("(The proof reveals nothing about which voter cast it)")
"#,
    ),
    (
        "src/ballot.ach",
        r#"// Ballot registry utilities

export fn register_voters(keys) {
    mut commits = []
    for k in keys {
        commits.push(poseidon(k, 0p0))
    }
    return commits
}

export fn build_registry(commits) {
    let n0 = poseidon(commits[0], commits[1])
    let n1 = poseidon(commits[2], commits[3])
    return poseidon(n0, n1)
}
"#,
    ),
];
