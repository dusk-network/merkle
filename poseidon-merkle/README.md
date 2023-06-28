# dusk-merkle

A sparsely populated [`dusk_merkle`](https://docs.rs/dusk-merkle/latest/dusk_merkle/)
merkle tree, which uses the poseidon hash algorithm for level aggregation and
is parametrized over its height and arity.
```text
Height 0             h
                    / \
                   /   \
                  /     \
                 /       \
                /         \
Height 1       h           h
              / \         / \
             /   \       /   \
Height 2    h     x     h     h
           / \         / \   / \
Height 3  h   x       x   h h   h
Position  0               5 6   7
```

Additionally to the tree itself, this crate defines an opening gadget that can
be used to create a merkle opening circuit for zero-knowledge applications
under the `"zk"` feature.

The type `Item<T>` has the aggregation of the `hash` part with the poseidon hash
pre-defined and additionally allows for a custom data type with custom
aggregation.

## Benchmarks

There are benchmarks for the poseidon tree calculation available with
```shell
cargo bench
```

and additional benchmarks for the opening proof generation with PLONK
```shell
cargo bench --features zk
```

This requires a nightly toolchain.

## License

This project is licensed under the Mozilla Public License, version 2.0. See the
[license](./LICENSE) file for more details.
