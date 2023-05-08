# Calmare & Themelios

## 0.1.3 (TBA)
- Docs
  - Instruction table now shows signature on hover.
  - Add docs to some instructions.
  - Add a page on text formatting.
- In ED7, show matrix decomposition on triggers
- Breaking
  - Add `item_use` to ed7scena header, instead of it being `unk`.
    - Also rename ed6scena's `item` to `item_use` because it's clearer.
  - Instruction set changes:
    - Merge `ObjFrame` and `ED7ObjFrame`.
    - Change signature of `ED7ObjPlay`.
    - Rename `ED7_6F` to `CamWait`
    - Rename `ObjPlay` to `ED6ObjPlay`.
    - Rename `Sc_Char98*` to `CharPath*`.
    - Rename `CharFlag2Set` to `CharFlags2Set`.
    - Merge `ED7_79` into `ObjWait`.
    - Merge `ED7_7D` into `MapColor`.
    - Change ScMenuSetTitle to `TString` instead of `Text`.
    - Fill in types for a bunch of unknown instructions.
  - Themelios internals:
    - Change `Text` from `Vec<TextSegment | Page>` to `Vec<Vec<TextSegment>>`.
    - Change most ::read and ::write functions to be associated.
    - Change `CharId` to an enum
    - Use Glam types where appropriate

- Bug fixes
  - Write `{item[n]}`, not `{item item[n]}`. (The latter remains valid syntax.)

## 0.1.2 (2023-03-20)
- Docs
  - Merge the Bracer Notebook and the other WIP book I was working on.
- Breaking
  - Backslash-newline is now for code formatting only, rather than representing NISA newline. For
    NISA newline, use `{0x0A}`.
  - The `VisSet*` instructions were replaced with a single dependently-typed `VisSet` instruction.
  - The `Emote` instruction no longer has a `emote[]` syntax, since that was just a silly
    syntactic special case.
  - `char_attr[char, attr]` is now written as `char.attr`.
- Bug fixes
  - Only print `{}` in dialogue at the start of a line.
  - Use Sky Evo's `visual/dt4` and `visual/dt24` pseudo-indexes, containing images.
    - The `dat/` directory, corresponding to archives 10 and 30 and containing battle data, does
      not have any index file (files were instead renamed, destroying their original names). These
      are left as `file[0x...]`.


## 0.1.1 (2023-03-12)
- Actually get full roundtripping working.
  - Which involves a number of breaking changes, not listed here.

## 0.1.0 (2023-03-10)
- Initial release.

# Cradle

## 0.3.0 (2023-05-08)
- Faces are 1555, not 4444
- c\_vis225 is 128×64
- itp32: Support swizzle mode 4
  - this is used in a few images in Zero/Azure, and also in several Ys games

## 0.2.1 (2023-04-17)
- Fix a number of issues concerning itp32

## 0.2.0 (2023-04-08)
- Support itc with non-contiguous frames.
- Use JSON instead of CSV.
- When possible, bake offset into extracted images.
- Support ED6 formats too (via separate exe).

## 0.1.2 (2023-03-20)
- Initial release.
