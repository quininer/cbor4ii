# CBOR 0x(4+4)9 0x49

“The Concise Binary Object Representation (CBOR)
is a data format whose design goals include the possibility of extremely small code size,
fairly small message size, and extensibility without the need for version negotiation.”

see [rfc8949](https://www.rfc-editor.org/rfc/rfc8949.html)

## Compatibility

The `core` mod should be fully compatible with rfc8949,
but some extensions will not be implemented in this crate,
such as `datetime`, `bignum`, `bigfloat`.

The `serde` mod defines how Rust types should be expressed in CBOR,
which is not any standard,
so different crate may have inconsistent behavior.

This library is intended to be compatible with `serde_cbor`,
but will not follow some unreasonable designs of `serde_cbor`.

* `cbor4ii` will express the unit type as an empty array instead of null.
This avoids the problem that `serde_cbor` cannot distinguish between `None` and `Some(())`.
see https://github.com/pyfisch/cbor/issues/185
* `cbor4ii` does not support packed mode, and it may be implemented in future,
but it may not be compatible with `serde_cbor`.
If you want packed mode, you should look at `bincode`.

# License

This project is licensed under [the MIT license](LICENSE).
