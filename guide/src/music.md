# Step 5: Music

Beyond the creativity aspect of making custom scenes, we can also convey
messages and emotions, serious or lighthearted. Music can be important in
setting the right mood for our scenes and ensure that the intended feeling is
coming across. Generally speaking, all areas of Liberl will have their own
themes. However, depending on the scene, the music may change or cut out
altogether. We can customize our own BGM to create the proper feel for our
scenes!

Let's take a look at `t0121` for the scene in Rolent's Bracer Guild, when Aina
informs Estelle, Joshua, and Schera that Cassius has gone missing.

![A scene of Aina talking on the phone. We only see one side of the conversation.](./img/music1.gif)

At the beginning of the scene we have the usual Rolent theme, [*Provincial City
of Rolent*][BGM_Rolent]. However, when Aina receives the bad news, the music
gradually fades out. After a brief silence, the music switches to [*People
Engaged in Secret Maneuvers*][BGM_Plot]. Here's the scene again, this
time with subtitles to indicate the playing audio.

![The same scene, but with annotations for the BGM that is playing.](./img/music2.gif)

```clm
	TextTalk char[0] {
		#21716v#743F#6PYes, that's right. He left on
		business the other day...{wait}
	}
	TextWait
	BgmStop 1500ms
	Emote char[0] 0mm 2000mm emote[24,27,250ms,0]
	Sleep 1500ms
	BgmWait
	EmoteStop char[0]
	TextTalk char[0] {
		#21717v#742F#3S#6PWhat?!{wait}
	}
	Shake 0 200 3000 100ms
	TextWait
	CharTurnToChar name[1] char[0] 400deg/s
	CharTurnToChar name[0] char[0] 400deg/s
	BgmPlay bgm[81] 0
	Sleep 400ms
	TextTalk char[0] {
		#21718v#745F#6PI apologize, but this is a little
		difficult to believe...{wait}
	} { ... }
```

Here, the instructions to look for are `BgmStop`, `BgmWait`, and `BgmPlay`.

`BgmStop` makes the music fade out over the specified duration. This often
feels more organic than having it just stop suddenly -- though a sudden stop
could also be effectful sometimes.

`BgmWait` is similar to `FadeWait`, as discussed in the previous chapter. It
makes the script wait until the `BgmStop` has finished -- it may otherwise
feel janky to have the scene progress while the audio is still fading. out.

Finally, `BgmPlay` tell the game to start playing a new track. The way these
IDs map to filenames is done through a file called `t_bgmtbl._dt`. Calmare does
not currently handle this file, though this is intended to be added in the future.

However, all hope is not lost: the Evolution versions of the games include a
more readable form of these tables, which I have included here. (Click to
expand.)

<details><summary><em>Trails in the Sky FC</em></summary>

```
{{#include bgmtbl/fc.txt}}
```
[(Raw)](bgmtbl/fc.txt)

</details>
<details><summary><em>Trails in the Sky SC</em></summary>

```
{{#include bgmtbl/sc.txt}}
```
[(Raw)](bgmtbl/sc.txt)

</details>
<details><summary><em>Trails in the Sky the 3rd</em></summary>

```
{{#include bgmtbl/3rd.txt}}
```
[(Raw)](bgmtbl/3rd.txt)

</details>

Each of the files consists of two parts: lines starting with `#define`, and
lines with `bgmtbl`. In this case we can see from the `#define` lines that
`music[81]` is called `BGM_Plot`, and from the `bgmtbl` line we know that this
is the file `BGM/ED6511.ogg`. We can tell from [the comment][jisho] that this
track is meant to represent plots, conspiracy, and uneasiness. The names such
as `BGM_Plot` are not used in Calmare, but can be useful as a reference.

As a further convenience, here's a table with a small excerpt of the tracks.

|ID|Falcom Name|Filename|Soundtrack Name|
|:-|:-|:-|:-|
|`music[1]` |`Title`     |ED6001|[*Feelings Soar with the Wind*][BGM_Title]|
|`music[10]`|`Rolent`    |ED6100|[*Provincial City of Rolent*][BGM_Rolent]|
|`music[16]`|`Sekisyo`[^sekisyo]   |ED6106|[*Border Patrol Ain't Easy*][BGM_Sekisyo]|
|`music[20]`|`Field`     |ED6200|[*The Way They Walk in Liberl*][BGM_Field]|
|`music[30]`|`Cave`      |ED6300|[*Prowling in the Dark*][BGM_Cave]|
|`music[32]`|`Cave2`     |ED6302|[*Tranquility Bestowed By Twilight*][BGM_Cave2]|
|`music[40]`|`Battle00`  |ED6400|[*Sophisticated Fight*][BGM_Battle00]|
|`music[48]`|`GameOver`  |ED6408|[*Disappearing Star*][BGM_GameOver]|
|`music[70]`|`Harmoni`   |ED6500|[*Whereabouts of Light Harmonica short Ver.*][BGM_Harmoni]|
|`music[72]`|`ED6502`    |ED6502|[*Amber Amour Piano Ver*][BGM_ED6502]|
|`music[81]`|`Plot`      |ED6511|[*People Engaged in Secret Maneuvers*][BGM_Plot]|
|`music[87]`|`Airpirates`|ED6517|[*We are the Capua Family!*][BGM_Airpirates]|
|`music[93]`|`Drama1`    |ED6530|[*Madrigal of the White Magnolia - Princess' Trouble*][BGM_Drama1]|

That's all there is to music! While the intricacies of finding the ID to play
the right song may be a bit tricky, getting music to play at the right times is
relatively simple! With this new tool at our disposal, we can set the proper
ambience of our scenes!

[BGM_Title]:      https://youtu.be/2f0pwOWgWg0&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=57
[BGM_Rolent]:     https://youtu.be/DukAeM4IytQ&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=4
[BGM_Sekisyo]:    https://youtu.be/xZOiCAk2kcM&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=15
[BGM_Field]:      https://youtu.be/DdgUSZoqmTc&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=13
[BGM_Cave]:       https://youtu.be/nMliPP6lUgc&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=35
[BGM_Cave2]:      https://youtu.be/c14T64KUDX4&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=38
[BGM_Battle00]:   https://youtu.be/5nBJzD4dFGY&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=6
[BGM_GameOver]:   https://youtu.be/EcpUIKxyjrM&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=22
[BGM_Harmoni]:    https://youtu.be/KZvVVr-W1s4&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=2
[BGM_ED6502]:     https://youtu.be/KMZj50w20yk&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=42
[BGM_Plot]:       https://youtu.be/kgWUHrTl0RM&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=40
[BGM_Airpirates]: https://youtu.be/_g6aGj1sCs0&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=19
[BGM_Drama1]:     https://youtu.be/VIUO9346owg&list=OLAK5uy_kqOOW4j2MqqtNhYnVFMyFTbCqNUHyoaU4&index=24

[jisho]: https://jisho.org/search/陰謀、悪巧み、不安
[^sekisyo]: 関所 means border checkpoint.
