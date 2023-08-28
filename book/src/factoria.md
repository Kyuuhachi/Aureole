# Industrial Block - Factoria
> Factoria is Liber Ark's beating industrial heart, located on
> the eastern side of the Ark. All daily necessities, from
> food to clothes to medicines to even homes, are produced in
> Factoria's round-the-clock workshops.
>
> The district is made up of 64 blocks, with blocks one
> through eight designed as the Ark's primary trade port.

Factoria is an application for manipulating the `.dir`/`.dat` archives used in
the PC versions of the Sky games.

The most important functionality, of course, is extracting the constituent
files from an archive. This can be done simply by dragging the `.dir`[^dir]
file over the exe, which will place the files in a subdirectory of the same
name next to it.

Other functionality, only available on the command line, includes:
- listing file contents,
- adding and deleting files[^optimize],
- creating archives from scratch (not yet implemented).

However, **use of these features is largely discouraged:** there is very little
benefit in creating or editing archives compared to using [LB-ARK](./lb-ark.md).

[^dir]: Dragging the `.dat` file currently does not work. This may change in the future.
[^optimize]:
  For performance reasons, adding and deleting files will avoid affecting other
  entries of the archive. Deleted or relocated entries will be zeroed out, so
  there's no risk of accidentally leaking deleted file data, but the space it
  previously occupied is still there, as well as other evidence of the edit
  history. Before publishing an archive, it is therefore recommended to use the
  `defrag` subcommand to eliminate this unused space.
