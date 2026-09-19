# textwrap

Wrap, fill, indent and dedent text for a terminal, in
[Meadow](https://github.com/mcdearman/meadow).

This package is a port of Rust's [`textwrap`](https://github.com/mgeisler/textwrap)
0.16.2 with its default features:

- widths come from [unicodeWidth](https://github.com/mcdearman/MeadowUnicodeWidth),
  so wide CJK characters and emoji are measured correctly and colour codes are
  ignored;
- words end where [unicodeLinebreak](https://github.com/mcdearman/MeadowUnicodeLinebreak)
  (UAX #14) says a line may break;
- the default algorithm is *optimal fit*, which balances line lengths across a
  whole paragraph, like TeX does, instead of filling each line greedily.

## Install

```sh
meadow add mcdearman/MeadowTextwrap
```

## Use

```meadow
use Textwrap (wrap, fill, indent, dedent, options, FirstFit)

def text = "Memory safety without garbage collection."

def main =
  ( wrap (options 18) text,
    -- ["Memory safety", "without garbage", "collection."]
    fill { options 18 | initialIndent = "* ", subsequentIndent = "  " } text,
    -- "* Memory safety\n  without garbage\n  collection."
    wrap { options 18 | wrapAlgorithm = FirstFit } "To be, or not to be, that is the question.",
    -- ["To be, or not to", "be, that is the", "question."]
    dedent "    def f\n      body\n"
    -- "def f\n  body\n"
  )
```

`options width` gives the crate's defaults. Change any field with record
update syntax:

| field | default | |
|---|---|---|
| `width` | the argument | the widest a line may be, in columns, indents included |
| `lineEnding` | `LF` | `LF` or `CRLF`; used both to split the input and to join `fill`'s output |
| `initialIndent` | `""` | put before the first line |
| `subsequentIndent` | `""` | put before every other line |
| `breakWords` | `True` | break a word that is wider than a line |
| `wrapAlgorithm` | `OptimalFit defaultPenalties` | or `FirstFit`; the `Penalties` record tunes optimal fit |
| `wordSeparator` | `UnicodeBreakProperties` | or `AsciiSpace`, or `CustomSeparator f` |
| `wordSplitter` | `HyphenSplitter` | or `NoHyphenation`, or `CustomSplitter f` |

| function | |
|---|---|
| `wrap opts text` | the lines, as a vector |
| `fill opts text` | the lines joined with `opts.lineEnding` |
| `fillInplace width text` | turns spaces into line breaks, so the result is the same length in bytes |
| `refill opts text` | fills already-filled text again, keeping its indents |
| `unfill text` | joins filled text back into one line, returning `(text, the options it was filled with)` |
| `wrapColumns n opts left middle right text` | lays text out in `n` side-by-side columns |
| `indent prefix text` | puts `prefix` before every line |
| `dedent text` | removes the whitespace that every line starts with |
| `displayWidth text` | the columns `text` takes, not counting escape sequences |

The building blocks are exported too: `findWords`, `splitWords`, `breakWords`,
`wrapFirstFit` and `wrapOptimalFit`. With them you can wrap your own fragments.

Not ported:

- `termwidth`, because Meadow has no way to ask for the terminal size yet;
- the `hyphenation` feature, which needs dictionaries.

## How it's made

The code in `src/` is a hand translation of textwrap's source, plus the
column-minima search (SMAWK) from the `smawk` crate. The translation keeps
their order of floating-point operations and tie-breaking, because optimal fit
depends on both.

**`src/Cases.mw`** is generated test data: about 7,000 random texts and option
settings. The texts mix words, hyphens, spaces, line endings, wide
characters, emoji and terminal escapes. Each case records what the crate
returns for `wrap`, `fill`, `fillInplace`, `indent`, `dedent`, `unfill`,
`refill`, `wrapColumns` or `displayWidth`, and `meadow test` checks that this
port gives the same result for every one. Cases where the crate panics are
left out.

To regenerate, run `scripts/generate.sh`. It needs a Rust toolchain. If
textwrap or smawk changed, the generator stops so that `src/` can be updated
first.

## Licence

MIT, like textwrap and smawk. See [LICENSE](LICENSE) and [COPYRIGHT](COPYRIGHT).
