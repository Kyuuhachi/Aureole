# Appendix A: Text Formatting

The Trails games use a variety of directives for various types of text formatting, most with the
format of a `#` sign, some numbers, and the one letter. It is not fully known which ones exist, or
which are supported by each game or in each context. Regardless, the known ones are listed below.

Some images would perhaps be useful. Should add some later.

## Brace directives

These are in actuality written as binary data in the text stream, so the syntax here is specific
to Calmare.

- `{wait}`: waits for a keypress before continuing with the rest of the text. Almost always placed
  at the end of a page. Official name is `PAUSE`.
- `{color _}`: Sets the color of the following text. Color persists even if you close the text
  box, so don't forget to put it back to default.

  <div style="columns:2; margin: 0 10%">

  |Number|Color|
  |:-:|:-:|
  |0|<span style=color:#FFF>COL\_TEXT|
  |1|<span style=color:#FC7>COLOR|
  |2|<span style=color:#F52>COL\_ITEM|
  |3|<span style=color:#8CF>COLOR|
  |4|<span style=color:#FB4>COL\_GOLD|
  |5|<span style=color:#8F9>COL\_SYSTEM|
  |6|<span style=color:#888>COLOR|
  |7|<span style=color:#FEE>COLOR|
  |8|<span style=color:#8F3>COLOR|
  |9|<span style=color:#333>COLOR|
  |10|<span style=color:#CA8>COLOR|

  |Number|Color|
  |:-:|:-:|
  |11|<span style=color:#FDB>COLOR|
  |12|<span style=color:#ACE>COLOR|
  |13|<span style=color:#CCF>COLOR|
  |14|<span style=color:#56B>COLOR|
  |15|<span style=color:#632>COLOR|
  |16|<span style=color:#135>COLOR|
  |17|<span style=color:#357>COLOR|
  |18|<span style=color:#BBB>COLOR|
  |19|<span style=color:#000>COLOR|
  |20|<span style=color:#BFB>COLOR|
  |21|<span style=color:#FBB>COLOR|

  </div>

  This table is based on *Trails in the Sky the 3rd*, PC versions. Values may vary for different games.

- `{item[_]}`: Displays the icon and name of an item, in color
  <span style=color:#F52>COL\_ITEM</span>. Always followed by a `{color 0}`, though this appears
  to be unnecessary. Can also be written as `#_i`.

- `{0xNN}`: Represents a raw byte, which generally has unknown meaning.

- `} {`: starts a new text page. Official name is `CLR`, presumably short for "clear".

## Hash directives

These directives are written as text in the binary files, so the syntax here is canonical. The
mnemonics, however, are not attested, and are purely informational.

- `#_F`: (Face) Sets the character portrait in the text box[^kao]. Also used for images in
  newspapers. `#F` without any numbers removes the portrait.

- `#_P`: (Position) Sets the position of the text box:

  |Number|Position|
  |-:|:-|
  |0|Auto|
  |1|Top left|
  |2|Top right|
  |3|Bottom left|
  |4|Bottom right|
  |5|Top center, pip pointing right|
  |6|Bottom center, pip pointing right|
  |7|Bottom center, pip pointing left|
  |8|Unknown, seems same as 0|
  |9|Bottom center, pip on bottom, pointing left|
  |10|Bottom center, pip on bottom, pointing right|
  |11|Top center, pip pointing left|
  |100|Same as auto, but does not move when character does|

- `#_W`: (Wait) Sets the text speed, in milliseconds per character. `#W` resets to default.

- `#_C`: (Color) Sets the text color, same as `{color _}`. See that one, above, for a color list.

- `#_S`: (Size) Sets the text size. Normal size is `#3S`. `#S` seems to make the text invisible.
  In *Sky*, this text color persists between pages, and will affect the character's name.

- `#_i`: (Item) Shows an item name and icon, same as `{item[_]}`.

- `#_I`: (Icon) Shows an icon. Which icons are available varies greatly between games.

- `#_M`: Seems to show an icon too. Details are unknown.

- `#_K`: Prevents the next `{wait}` instruction from proceeding until a `TextClose 1` instruction
  has been executed.

- `#_N`: Unknown. Only exists in ED7.

- `#_A`: (Auto) Makes the next `{wait}` directives proceed automatically after the given number of
  frames after it is shown. It is reset at the next `{wait}`, or `#A` can be used to reset it
  directly. There seems to be little reason to do the latter, though.

- `#_R_#`: (Ruby) This one's got a slightly different syntax. It is used for [ruby text][ruby],
  small characters sometimes used in Japanese text. The number specifies how far back the text is
  to be put, with two units per kanji. For example, `夏休#4RWelp.#` is rendered as
  <ruby>夏<rt></rt>休<rt>Welp.</ruby>. The exact details of how it looks may vary between games.

- `#_V`: (Voice) Sets a voice clip to play along the dialogue. Depending on the game, a mod may
  be required for this.

### Lip sync

These directives are exclusive to the *Evolution* versions, and are therefore largely unknown.

- `#_T`: (口パクテンポ) Lip sync tempo.
- `#_B` and `#_Z`: (口パク開始指定) Lip sync start and end, in number of frames after the voice clip started.
- `#_D`: (半開き) Notes in the debug script mention `Y` texture variants.
- `#_O`: (口空き) Notes in the debug script mention `Z` texture variants.
- `#_L`: (アニメーション停止) Unknown.
- `#_E`: (目開け) Unknown.
- `#_H`: Unknown.
- `#_U`: Unknown.

[^kao]: In *Sky*, files are looked up via `t_face`, but effectively resolve to `h_kao###._ch`, or
  `h_ka####._ch` for larger numbers. In Crossbell, they resolve to `ka#####.itp`.

[ruby]: https://en.wikipedia.org/wiki/Ruby_character
