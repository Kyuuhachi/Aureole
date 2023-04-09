# Industrial Block - Factoria
> Factoria is Liber Ark's beating industrial heart, located on
> the eastern side of the Ark. All daily necessities, from
> food to clothes to medicines to even homes, are produced in
> Factoria's round-the-clock workshops.
>
> The district is made up of 64 blocks, with blocks one
> through eight designed as the Ark's primary trade port.

Factoria is a work-in-progress application for manipulating the `.dir`/`.dat`
archives used in the PC versions of the Sky games.

Current functionality is limited to listing archive contents, but planned
features include extracting (full and selective), creating, inserting, replacing, and optimizing[^optimize] archives.

[^optimize]:
  Replacing files inside an archive is not possible if the file is larger than
  the original, so they will instead be added to the end of the archive, with
  the original data left behind. The optimize subcommand will put files back
  into order and eliminate this junk data.
