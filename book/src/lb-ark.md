# Substratum Access Passages — LB-ARK
> In the event of service disruption of the Halo Rail, a gate may be unlocked
> at each station, leading to substratum access passages. In accordance with
> emergency protocols, this station's gate can be unlocked.
>
> Would you like to do so at this time?

The Liberl Archive Redirection Kludge is a dll hook for the *Trails in the Sky*
trilogy. Its primary purpose is to allow the games to read data from loose
files rather than its file archives, alleviating the need for editing these
archives (via [Factoria](./factoria.md) or other tools).

This allows a faster development cycle — both by cutting out the repacking
step, and in many cases allowing files to be edited without even restarting the
game[^live-reload] — and improving mod distribution with smaller download size
and fewer conflicts with other mods.

Due to its different nature compared to the other components, it is hosted in a
[different repo][lb-ark].

## Usage

To install, download [`d3dxof.dll`] and place it next to the game's exe file,
normally in `C:\Program Files (x86)\Steam\steamapps\common\Trails in the Sky FC`
and siblings. If you use [SoraVoice], which contains its own, less powerful
file redirection facility, it is recommended to run [`move_sora_voice.ps1`] to
move this to LB-ARK's directories to avoid conflicts.

All files related to LB-ARK are inside the `{game directory}\data` directory.
This will be automatically created on startup. The most basic usage, which I
call *implicit overrides*, is to place files at `data\{archive_name}\{filename}`,
for example `data\ED6_DT01\t0310._sn`[^8.3], which will cause this file to be
loaded instead of the `t0310._sn` inside `ED6_DT01.dat`. If you used the
SoraVoice migration script, this will contain a number of files in the correct
structure, giving an example of the structure.

For more complex uses, such as creating new files, or simply putting all your
mod's files in one folder[^one-folder], create a `data\*.dir` file,
containing a json object:

```json
{
  "t0311._sn": "my-mod/t0311._sn",
  "0x00010098": "my-mod/file.bin",
  "0x00010099": { "name": "myfile.bin", "path": "my-mod/file.bin" }
}
```

These are all examples of *explicit redirects*. The first line simply tells
that the given preexisting file should be read from that path, rather than from
the archive or an implicit redirect. The latter two lines insert files that are
not already present in the base archives. The way *Sky* is structured, some
file accesses are done via *file ids* rather than filenames, so you need to
explicitly provide one. Any number works, as long as it does not conflict with
any other game, and is less or equal to 0x003FFFFF[^3fffff].

In addition, LB-ARK allows loading DLL files, from `data\plugins\*.dll`.
If a function `#[no_mangle] extern "C" fn lb_init();` exists, it will be called.
This is unrelated to archive redirection, but it's useful enough to be included.

[lb-ark]: https://github.com/Kyuuhachi/LB-ARK/
[`d3dxof.dll`]: https://github.com/Kyuuhachi/LB-ARK/releases/latest
[`move_sora_voice.ps1`]: https://raw.githubusercontent.com/Kyuuhachi/LB-ARK/main/move_sora_voice.ps1
[SoraVoice]: https://github.com/ZhenjianYang/SoraVoice

[^live-reload]:
  Whether a file can be live reloaded depends on whether the game loads that
  file once at startup, or on demand.What files live reloading works on depends
  on whether the game loads the file on startup or not. Broadly speaking, files
  that there are many of, such as textures or scenas, work fine, while unique
  ones like `t_item` do not. In particular, LB-DIR's own `.dir` files are not
  reloaded, so adding brand new files always requires a restart.
[^8.3]:
  You may have noticed that certain tools, including SoraVoice and ED6Unpacker,
  use filenames such as `T0310␣␣␣._SN`. LB-ARK, like the rest of the Aureole
  Suite, uses the natural form `t0310._sn` instead. To aid interoperability
  with these other tools, if the `$LB_DIR_SPACED_FILENAMES` is nonempty, LB-ARK
  will load files with this format as well. This is not recommended for
  distribution, however.
[^one-folder]:
  Putting all your mod's files in one directory makes it easier not only to
  distribute, but also to uninstall and to detect conflicts between mods.
[^3fffff]:
  In vanilla, file ids are effectively limited to about 2000-4000 per archive.
  LB-ARK releases this restriction, allowing for a continuous range up to 0x3FFFFF.
  Raising the limit more than this would be difficult, however.
