// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_plonk::prelude::*;
use dusk_poseidon::{Domain, Hash};
use ff::Field;
use poseidon_merkle::zk::opening_gadget;
use poseidon_merkle::{ARITY, Item, Opening, Tree};
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

// set max circuit size to 2^15 gates
const CAPACITY: usize = 15;

// set height of the poseidon merkle tree
const HEIGHT: usize = 17;

type PoseidonItem = Item<()>;

// Create a circuit for the opening
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
struct OpeningCircuit {
    opening: Opening<(), HEIGHT>,
    leaf: PoseidonItem,
}

impl Default for OpeningCircuit {
    fn default() -> Self {
        let empty = Item {
            hash: BlsScalar::zero(),
            data: (),
        };
        let mut tree = Tree::new();
        tree.insert(0, empty);
        let opening = tree.opening(0).expect("There is a leaf at position 0");
        Self {
            opening,
            leaf: empty,
        }
    }
}

impl Circuit for OpeningCircuit {
    fn circuit(&self, composer: &mut Composer) -> Result<(), Error> {
        // append the leaf and opening gadget to the circuit
        let leaf = composer.append_witness(self.leaf.hash);
        let computed_root = opening_gadget(composer, &self.opening, leaf);

        // append the public root as public input to the circuit
        // and ensure it is equal to the computed root
        let constraint = Constraint::new()
            .left(-BlsScalar::one())
            .a(computed_root)
            .public(self.opening.root().hash);
        composer.append_gate(constraint);

        Ok(())
    }
}

// The offsets follow the byte layout of `Opening::to_var_bytes`. The callers
// check the result through the accessors, so a change of that layout fails
// the test instead of tampering with a different item.
fn tamper_hash(
    opening: &Opening<(), HEIGHT>,
    item_index: usize,
    hash: BlsScalar,
) -> Opening<(), HEIGHT> {
    let offset = BlsScalar::SIZE * item_index;
    let mut bytes = opening.to_var_bytes::<32>();
    bytes[offset..offset + BlsScalar::SIZE].copy_from_slice(&hash.to_bytes());

    Opening::from_slice::<32>(&bytes)
        .expect("Tampered hash should remain canonical")
}

fn tamper_root(opening: &Opening<(), HEIGHT>) -> Opening<(), HEIGHT> {
    let hash = opening.root().hash + BlsScalar::one();
    let tampered = tamper_hash(opening, 0, hash);

    assert_eq!(tampered.root().hash, hash, "The root should be tampered");
    assert_eq!(tampered.branch(), opening.branch());
    assert_eq!(tampered.positions(), opening.positions());
    tampered
}

fn tamper_sibling(opening: &Opening<(), HEIGHT>) -> Opening<(), HEIGHT> {
    let level = HEIGHT - 1;
    let sibling = (opening.positions()[level] + 1) % ARITY;
    let item_index = 1 + level * ARITY + sibling;
    let hash = opening.branch()[level][sibling].hash + BlsScalar::one();
    let tampered = tamper_hash(opening, item_index, hash);

    let mut branch = *opening.branch();
    branch[level][sibling].hash = hash;
    assert_eq!(tampered.branch(), &branch, "The sibling should be tampered");
    assert_eq!(tampered.root(), opening.root());
    assert_eq!(tampered.positions(), opening.positions());
    tampered
}

fn assert_rejected(
    prover: &Prover,
    rng: &mut StdRng,
    opening: Opening<(), HEIGHT>,
    leaf: PoseidonItem,
    reason: &str,
) {
    assert!(
        !opening.verify(leaf),
        "{reason} should fail native verification"
    );
    assert!(
        prover
            .prove(rng, &OpeningCircuit { opening, leaf })
            .is_err(),
        "{reason} should fail proof generation"
    );
}

#[test]
fn opening_gadget_accepts_valid_and_rejects_tampering() {
    let label = b"merkle opening";
    let mut rng = StdRng::seed_from_u64(0xdea1);
    let pp = PublicParameters::setup(1 << CAPACITY, &mut rng).unwrap();

    let (prover, verifier) = Compiler::compile::<OpeningCircuit>(&pp, label)
        .expect("Circuit should compile successfully");

    let mut tree = Tree::new();
    let mut leaf = PoseidonItem::new(BlsScalar::zero(), ());
    let mut position = 0;
    for _ in 0..100 {
        let hash =
            Hash::digest(Domain::Other, &[BlsScalar::random(&mut rng)])[0];
        position = rng.next_u64() % tree.capacity();
        leaf = PoseidonItem::new(hash, ());
        tree.insert(position, leaf);
    }
    let opening = tree.opening(position).unwrap();
    assert!(opening.verify(leaf));

    let circuit = OpeningCircuit { opening, leaf };

    let (proof, public_inputs) = prover
        .prove(&mut rng, &circuit)
        .expect("Proof generation should succeed");

    // The root is the only public input, so the verifier checks the proof
    // against the root and not against a value the prover picks.
    let root = opening.root().hash;
    assert_eq!(public_inputs, [root], "The root should be the public input");

    verifier
        .verify(&proof, &[root])
        .expect("Proof verification should succeed");

    let tampered_root = tamper_root(&opening);
    assert!(
        matches!(
            verifier.verify(&proof, &[tampered_root.root().hash]),
            Err(Error::ProofVerificationError)
        ),
        "The proof should not verify against a tampered root"
    );

    let wrong_leaf = PoseidonItem::new(leaf.hash + BlsScalar::one(), ());
    assert_rejected(&prover, &mut rng, opening, wrong_leaf, "Wrong leaf");

    let tampered_sibling = tamper_sibling(&opening);
    assert_rejected(
        &prover,
        &mut rng,
        tampered_sibling,
        leaf,
        "Tampered sibling",
    );

    assert_rejected(&prover, &mut rng, tampered_root, leaf, "Tampered root");
}
