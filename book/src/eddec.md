# Versus EDDecompiler

It would be foolish to publish a script decompiler without comparing it to
the only other known tool in its niche: Ouroboros'
[EDDecompiler](https://github.com/GeofrontTeam/EDDecompiler/), or any of its
many forks. Calmare/Themelios has a number of improvements compared to
EDDecompiler:

## Structured code

Calmare is a decompiler. EDDecompiler is, despite the name, a disassembler. To
see the difference, compare the following decompilations of a random script
function (namely Azure's `e3020`, function 0):

### EDDecompiler ([Source](https://github.com/GeofrontTeam/EDDecompiler/blob/5017ec026ff7f96d1b22094e5fcd69821f176f04/Decompiler/p/scena/e3020.梅尔卡瓦.bin.py#L173-L238))

```py
    def Function_0_4EC(): pass

    label("Function_0_4EC")

    Switch(
        (scpexpr(EXPR_RAND), scpexpr(EXPR_PUSH_LONG, 0x8), scpexpr(EXPR_IMOD), scpexpr(EXPR_END)),
        (0, "loc_524"),
        (1, "loc_530"),
        (2, "loc_53C"),
        (3, "loc_548"),
        (4, "loc_554"),
        (5, "loc_560"),
        (6, "loc_56C"),
        (SWITCH_DEFAULT, "loc_578"),
    )


    label("loc_524")

    OP_A0(0xFE, 1450, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_530")

    OP_A0(0xFE, 1550, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_53C")

    OP_A0(0xFE, 1600, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_548")

    OP_A0(0xFE, 1400, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_554")

    OP_A0(0xFE, 1650, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_560")

    OP_A0(0xFE, 1350, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_56C")

    OP_A0(0xFE, 1500, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_578")

    OP_A0(0xFE, 1500, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_584")

    Jc((scpexpr(EXPR_PUSH_LONG, 0x1), scpexpr(EXPR_END)), "loc_59B")
    OP_A0(0xFE, 1500, 0x0, 0xFB)
    Jump("loc_584")

    label("loc_59B")

    Return()
```


### Calmare

```clm
fn[0]:
	switch random % 8:
		case 0:
			ED7_A0 self 1450ms 0 251
			break
		case 1:
			ED7_A0 self 1550ms 0 251
			break
		case 2:
			ED7_A0 self 1600ms 0 251
			break
		case 3:
			ED7_A0 self 1400ms 0 251
			break
		case 4:
			ED7_A0 self 1650ms 0 251
			break
		case 5:
			ED7_A0 self 1350ms 0 251
			break
		case 6:
			ED7_A0 self 1500ms 0 251
			break
		default:
			ED7_A0 self 1500ms 0 251
			break
	while 1:
		ED7_A0 self 1500ms 0 251
	Return
```

## Roundtripping

Great care is taken to ensure that the binary files written by Themelios are
identical to those produced by Falcom's own tools. While those produced by
EDDecompiler are *functionally* equivalent, they are often different
internally.

In particular, EDDecompiler omits parts of the code that the games do not
access: it traverses the control flow of the bytecode, and for example quits when
it encounters a `Return` instruction. I believe this is done because there is no
easy way to tell when the code ends, and trying to read past that end would be
troublesome. Themelios uses more clever heuristics so it does not have to do this,
and thus parses the exact sequence of instructions.

For an example, consider the following pseudocode snippet:

```clm
if flag[8]:
	var[0] = 0
	Return
else:
	var[0] = 1
	Return
```

This would compile to the bytecode sequence:

```clm
if flag[8] @l1
var[0] = 0
Return
goto @l2 // <-- unreachable
@l1
var[0] = 1
Return
@l2
```

EDDecompiler would deduce that the `goto @l2` is unreachable and remove it.
While this change does not matter to the game engine, it *does* affect
Themelios, which would give a slightly different decompilation:

```clm
if flag[8]:
	var[0] = 0
	Return
var[0] = 1
Return
```

This can make a large difference when working with scripts programmatically
with Themelios. In some cases, it can even eliminate interesting pieces of
code — for example in FC's `t0121_1`, Rolent's guild branch, there's over 300
lines of unused code manually implementing the quest reporting function, which
I believe EDDecompiler glosses over entirely.

## API layer

EDDecompiler does not provide any way to manipulate scripts programmatically.
It converts from binary to text, and from text to binary. If you need anything
else, you're on your own.

The Themelios Suite provides, well, Themelios for this purpose. Calmare is
a fairly thin layer on top of this, so anything Calmare does, any Rust
programmer can do too.

## Types

Themelios includes types on many instruction arguments, which comes with a
number of benefits:

- **Readability**. In my opinion, it is simply easier to see what the script
  does this way.
- **Searching**. Want to find all references[^item-ref] to a specific quest?
  `rg 'quest\[125\]'. Very useful for finding what I needed to patch for
  Inevitable Zero and Azure Vitality.
- **Manipulation**. Like above, you can do simple search-and-replace on terms.
  This also works programmatically: remapping `fn[]` or `char[]` terms came in
  real handy for my mods.

[^item-ref]: However this is not flawless: numbers inside expressions are not
  well-typed. For example, SC has some comparisons like
  `if field_party[0].31 == 337 | field_party[0].32 == 337:`
  for checking if a character is wearing a ZFG (char attrs 31 and 32 are the
  items worn in each accessory slot). This is something I would like to solve
  eventually.

## Instruction name stability

EDDecompiler names unknown instructions as `OP_NN`, where `NN` is the hex code
for the instruction. Since the instruction tables are different between each
game, this means that for example `OP_A0` might do completely different things
in different games. Themelios uses the same name for instructions that are
semantically the same, even if they are in different games. This is much easier
to crossreference between games, especially between the base and Evolution
games.

## Ease of use

I'll be honest with you: I've never run EDDecompiler. In part for the reasons
mentioned above, but also because I've never figured out the installation
procedure[^havent-tried]. In contrast, Calmare is just download the executable
and run, at least if you only need the basic features.

[^havent-tried] Okay I didn't try all that hard, either.
