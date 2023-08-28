# Core Sector - Themelios
> No, that's just some broadcast equipment.[^axis-pillar]
>
> It sends the miraculous power from the
> Core Sector out across the city.

> The proper function of the Axis Pillar is,
> essentially, as an antenna broadcasting the
> Aureole's power to the rest of the city.
>
> Its range seems to be roughly a thousand
> selge.
>
> Using the Gospels, the Aureole could affect
> things not just in what we call Liberl now, but
> across a good deal of the continent.

Themelios is a Rust library that powers Calmare, reading and writing binary
files into well-typed Rust objects. It is currently not particularly
well-documented, but it gets the job done.

API docs can be found [here](https://aureole.kyuuhachi.dev/doc/themelios/).

## Structure

Themelios consists of a sizable number of internal crates, but the ones
intended for end users are:

- `themelios` - Conversions between *Trails'* binary files and programmatically
  accessible data structures.
- `calmare` - Conversions between `themelios`'s data structures and Calmare's
  textual syntax. Way overdue for a significant refactoring tbh.
- `cradle` - Conversions between *Trails'* custom image formats and the `image`
  crate.

The crates suffixed `-cli` are for binaries. You can look at these for usage
examples, but for normal use, it's probably easier to just download the
prebuilt binaries.

The crates prefixed `themelios-` should not be referenced directly; the useful
parts are already reexported in the main `themelios` crate.

There are also a few small crates for specific subtasks that could in principle
be used standalone:

- `gospel` - Incremental byte munching, both for reading and writing.
- `cp932` - Conversions between codepage 932 and UTF-8; `encoding_rs` seems to
  implement a different shift-jis variant that is not compatible with *Trails*.
- `bc7` - For decoding BC7-compressed images. For compression, try `intel_tex_2`.
- `bz` - One of Falcom's proprietary compression algorithms.

## MSRV

Themelios makes use of a fairly large number of unstable features, so nightly
compiler is needed. These features include (but this list might not be complete):

- `decl_macro` - *So* much nicer than standard `macro_rules!`. Got some issues
  with overeager hygiene, though.
- `error_generic_member_access`, `provide_any` - Needed for `thiserror`. Could
  probably do without them, but since I'm already using nightly, who cares?
- `let_chains` - Some very nice QoL. Not strictly necessary, but again, why not.
- `try_blocks` - Can't believe this isn't in the language already.
- `slice_flatten`, `array_try_map`, `pattern` - Some small API improvements for `std`.
- `try_trait_v2`, `never_type`, `try_trait_v2_residual` - Used for a couple of
  small but useful combinators. Some of these could be rewritten to be less
  general without any notable loss, but my
  [`strict_result`](https://crates.io/crates/strict_result) also requires them.
- `proc_macro_diagnostic` - I could probably use the `proc_macro_error` crate instead.

[^axis-pillar]:
  Yes, I know that this quote is about the Axis Pillar, not the Core Sector.
  But "Themelios" sounds much cooler, so that's what it's named.
