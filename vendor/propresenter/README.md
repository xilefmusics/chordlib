# ProPresenter schema subset

`propresenter_minimal.proto` contains only the fields used by chordlib. Its
field numbers and wire types are copied from the reverse-engineered
[`greyshirtguy/ProPresenter7-Proto`](https://github.com/greyshirtguy/ProPresenter7-Proto)
schema at commit `1b63dda196eb7e079721a8a4a7e7773520cb5ad2` (retrieved
2026-07-20).

The schema is documentation for the checked-in `prost::Message` definitions in
`src/propresenter_proto.rs`; it is not compiled during a chordlib build.

The upstream schema is unofficial and unsupported by Renewed Vision. See
`LICENSE` for its MIT license.
