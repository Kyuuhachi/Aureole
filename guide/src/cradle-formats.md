# Cradle - File formats

This section lists some technical details about the different file formats. It
is not necessary for using Cradle, but is here in case anyone is interested.

## ED6


These formats are used by the *Trails in the Sky* trilogy. It is possible that
other games from that time period use these formats too, but even if so, I am
not aware of any.

### `._ch`

`._ch`, which possibly stands for "chip", is used for various kinds of images.
It is an exceedingly simple format, which consists solely of the raw ARGB byte
stream. This, counterintuitively, makes it one of the most difficult formats to
handle, because there is no data such as width or height, or whether the image
uses 1-bit, 4-bit, or 8-bit alpha. Yes, all three variants exist.

### `._ch` & `._cp`

This pair of files, where the `p` probably stands for "pattern", is used for
the animated sprites of characters and monsters, and also certain other objects
such as dinner plates.

Both files consist of a 16-bit number, followed by that many `u16×16×16`
tiles. In the `._ch`, these tiles are ARGB4444 images; for the `._cp`,
they are indices into the `._ch` array (or 0xFFFF for empty), forming a
sequence of 256×256 images.

From the exact arrangement of the tiles inside the `._ch`, a few interesting
pieces of trivia can be deduced:

- The tiles are not stored in a per-frame row-major order, but instead an order
  that would be row-major if the frames were laid out in a 8-wide grid. This
  corresponds to the eight possible rotations of most sprites.

- One would expect this scheme to be used for deduplicating tiles that are used
  multiple times in the spritesheet. And indeed, this is the case -- but far
  less frequently than what is possible; only a minuscule number of tiles with
  only a few semitransparent pixels at the edges are deduplicated, even though
  more is possible. This could possibly hint that the deduplication is done on
  32-bit images before reducing it down to 16-bit color.

  - That a few almost-blank are still deduplicated hints that the alpha channel
    is low precision in the source material, which could hint that the sprites
    are rendered with 16 samples per frame.

### Other

Some other lower-priority files that I may add support for later include:

`._x2`, `._x3`: 3D models\
`._ef`: (`.eff`) graphical effects, such as particles or magic.\
`._da`: font\
`._ct`, `._hd`: (`.oct`, `.shd`) Alternative representations for world
geometry, for collisions and shadows, respectively.\
`._lm`: 3D textures for position-dependent character shading

## ED7

These formats are used by many newer Falcom games, including *Trails from
Zero* and *to Azure*, all of the *Trails Evolution* games, *Zwei II*,
*Gurumin*, *Ys VII* and *VIII*, and *Legend of Nayuta*.

### `.itp` (Picture)

Like `._ch`, these are just everyday bitmap images, for a variety of use cases.
As anyone familiar with Falcom's practices would expect, there's a wide variety
of different formats.

The older versions all have a palette, which limits them to 256 different
colors. They are used in *Trails Evolution*, as well as the Joyoland versions
of *Trails from Zero* and *to Azure*.

**1000** - 256 colors palette, followed by raw image data. The data here is not
compressed.

**1002** - Same as 1000, but both the palette and image data are compressed.

**1004** - Variable palette size, and the image data is chunked into 16×8
chunks for better compression.

**1005** - The variable-size palette is now encoded as a delta, probably for
better compression. It is also chunked into 16×8 chunks, but now each chunk
has its own subset of the palette (max 16 colors per chunk), and uses only four
bits per pixel.

**1006** - Now we're talking. This format has it all -- a mysterious `CCPI`
header, variable-size chunking, flags -- just lovely. One of the flags tells
whether the image data is compressed. Another tells whether the file has an
embedded or external palette.

Oh, and each of the chunks has its own tileset of 2×2 tiles, with special
handling for tiles that are mirrors of each other, meaning effectively two bits
per pixel. And the tile assembly data is run-length encoded (in addition to the
usual compression). Who even comes up with this?

**ITP<sub>FF</sub>** - Or itp32, as I like to call it. This variant is used by the NISA
*Zero* and *Azure* ports, as well as *Ys VIII*, I believe. It is significantly
more sophisticated than the formats above. It is a TLV format, and supports
features such as mipmaps and BC7 compression. Actually I don't know if it
supports *not* using BC7. This one uses a completely different compression
algorithm than the earlier variants.

It should also be mentioned that some `.itp` files in Geofront are just renamed
PNG files. Guess they didn't like the 256 color limitation.

### `.itc` (Chip? Character?)

This fills the same role as the `._ch`&`._cp` combo of old. This one doesn't do
any kind of tiling, though; it's just a straight sequence of `.itp` files plus
some metadata. It is also not limited to 256×256 pixels, allowing for higher
sprite resolutions.

Another point of interest is the `V102` format. It's exactly the same as the
usual `V101` format, except it contains a palette, which is used by the
contained format 1006 `.itps`.

### Other

Again, lower priority, but I might get to them eventually.

`.it3`: 3D model\
`.ite`: effect\
`.itf`: font\
`.iti`: unknown
