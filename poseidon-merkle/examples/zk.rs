// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;
use dusk_poseidon::{Domain, Hash};
use ff::Field;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

use poseidon_merkle::zk::opening_gadget;
use poseidon_merkle::{
    Item as PoseidonItem, Opening as PoseidonOpening, Tree as PoseidonTree,
};

// set max circuit size to 2^16 gates
const CAPACITY: usize = 16;

// set height of the poseidon merkle tree
const HEIGHT: usize = 17;

// Create a circuit for the opening
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct OpeningCircuit {
    opening: PoseidonOpening<(), HEIGHT>,
    leaf: PoseidonItem<()>,
}

impl Default for OpeningCircuit {
    fn default() -> Self {
        let empty_item = PoseidonItem {
            hash: BlsScalar::zero(),
            data: (),
        };
        let mut tree = PoseidonTree::new();
        tree.insert(0, empty_item);
        let opening = tree.opening(0).expect("There is a leaf at position 0");
        Self {
            opening,
            leaf: empty_item,
        }
    }
}

impl OpeningCircuit {
    /// Create a new OpeningCircuit
    pub fn new(
        opening: PoseidonOpening<(), HEIGHT>,
        leaf: PoseidonItem<()>,
    ) -> Self {
        Self { opening, leaf }
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

fn main() {
    let label = b"merkle poseidon opening";
    let mut rng = StdRng::seed_from_u64(0xdea1);
    let pp = PublicParameters::setup(1 << CAPACITY, &mut rng).unwrap();

    let (prover, verifier) = Compiler::compile::<OpeningCircuit>(&pp, label)
        .expect("Circuit should compile successfully");

    let mut tree = PoseidonTree::new();
    let mut leaf = PoseidonItem::<()>::new(BlsScalar::zero(), ());
    let mut position = 0;
    for _ in 0..100 {
        let hash =
            Hash::digest(Domain::Other, &[BlsScalar::random(&mut rng)])[0];
        position = rng.next_u64() % u8::MAX as u64;
        leaf = PoseidonItem::<()>::new(hash, ());
        tree.insert(position as u64, leaf);
    }
    let opening = tree.opening(position as u64).unwrap();
    assert!(opening.verify(leaf.clone()));

    let circuit = OpeningCircuit::new(opening, leaf);

    let (proof, public_inputs) = prover
        .prove(&mut rng, &circuit)
        .expect("Proof generation should succeed");

    verifier
        .verify(&proof, &public_inputs)
        .expect("Proof verification should succeed");
}
