//! Writes `src/Tables.mw` and `src/Cases.mw` for textwrap.
//!
//! ```text
//! cargo run --release -- <package root>
//! ```
//!
//! * **Tables** -- which code points Rust's `char::is_alphanumeric` accepts,
//!   since textwrap's hyphen splitter asks it.
//! * **Cases** -- texts and options, with what textwrap makes of them:
//!   `wrap`, `fill`, `fill_inplace`, `indent`, `dedent`, `unfill`, `refill`,
//!   `wrap_columns` and `display_width`. The texts are built at random from
//!   words, spaces, hyphens, line endings, wide characters and terminal
//!   escapes, and the options cover every setting. Calls on which textwrap
//!   panics are left out.
//!
//! The library itself is ported by hand into `src/`. Its source is
//! fingerprinted, so that a new version of textwrap is not picked up
//! unnoticed.

use std::fmt::Write as _;
use std::path::PathBuf;
use textwrap::wrap_algorithms::Penalties;
use textwrap::{LineEnding, Options, WordSeparator, WordSplitter, WrapAlgorithm};

const SCALARS: u32 = 0x11_0000;

/// The crate version pinned in `Cargo.toml`.
const UPSTREAM_VERSION: &str = "0.16.2";

/// The fingerprint of textwrap's and smawk's sources, which `src/` ports.
const SOURCES: u64 = 0x115a_ac38_23c6_565c;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "../..".into()));

    let print = fingerprint(include_str!(concat!(env!("OUT_DIR"), "/sources.rs.txt")));
    if print != SOURCES {
        eprintln!(
            "error: textwrap is not the version src/ ports.\n\
             Compare its source in {} with the previous version, carry any change\n\
             into src/, then set SOURCES in scripts/generate/src/main.rs to\n\
             {print:#x}",
            env!("UPSTREAM_DIR")
        );
        std::process::exit(1);
    }

    // textwrap panics on some inputs (see `guard`); keep that quiet.
    std::panic::set_hook(Box::new(|_| {}));

    let tables = tables();
    let tables_path = root.join("src/Tables.mw");
    std::fs::write(&tables_path, &tables).unwrap();
    eprintln!("wrote {} ({} bytes)", tables_path.display(), tables.len());

    let cases = cases();
    let cases_path = root.join("src/Cases.mw");
    std::fs::write(&cases_path, &cases).unwrap();
    eprintln!("wrote {} ({} bytes)", cases_path.display(), cases.len());
}

/// FNV-1a: stable across builds, which `DefaultHasher` does not promise.
fn fingerprint(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

// --- encoding -----------------------------------------------------------------------

/// A number as `digits` base-64 digits, most significant first, each digit the
/// character `'0' + d`: `'0'` to `'o'`, one contiguous run of ASCII.
fn digits(out: &mut String, value: u64, digits: u32) {
    assert!(
        value < 1 << (6 * digits),
        "{value} does not fit in {digits} digits"
    );
    for k in (0..digits).rev() {
        out.push(char::from(b'0' + ((value >> (6 * k)) & 63) as u8));
    }
}

/// A string, as its length in bytes (3 digits) and then its bytes.
fn text(out: &mut String, s: &str) {
    digits(out, s.len() as u64, 3);
    out.push_str(s);
}

fn flag(out: &mut String, b: bool) {
    digits(out, u64::from(b), 1);
}

/// `text` as one Meadow string literal, broken with `\`-newline every `width`
/// characters. Only printable ASCII is written raw; a space that would start a
/// line is `\x20`, since a continuation drops leading whitespace.
fn long_literal(text: &str, width: usize) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / width * 4 + 2);
    out.push('"');
    for (i, c) in text.chars().enumerate() {
        let line_start = i > 0 && i % width == 0;
        if line_start {
            out.push_str("\\\n    ");
        }
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            ' ' if line_start => out.push_str("\\x20"),
            ' '..='~' => out.push(c),
            _ => {
                let _ = write!(out, "\\u{{{:X}}}", u32::from(c));
            }
        }
    }
    out.push('"');
    out
}

const HEADER: &str = "\
-- Copyright (c) 2016 Martin Geisler, and the Meadow port's authors. Licensed
-- under the MIT license: see LICENSE and COPYRIGHT.";

// --- tables -------------------------------------------------------------------------

fn tables() -> String {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for cp in 0..SCALARS {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        if !c.is_alphanumeric() {
            continue;
        }
        match runs.last_mut() {
            // The surrogates are never asked about, so a run may jump them.
            Some((_, hi)) if *hi + 1 == cp || (*hi == 0xD7FF && cp == 0xE000) => *hi = cp,
            _ => runs.push((cp, cp)),
        }
    }
    let mut body = String::new();
    for (lo, hi) in runs {
        digits(&mut body, lo.into(), 4);
        digits(&mut body, hi.into(), 4);
    }
    let (maj, min, pat) = char::UNICODE_VERSION;
    format!(
        "-- GENERATED by scripts/generate.sh for textwrap {UPSTREAM_VERSION}.
-- Do not edit: run the script again instead.
--
{HEADER}

-- The code points Rust's `char::is_alphanumeric` accepts (Unicode
-- {maj}.{min}.{pat}), as `lo hi` ranges, 4 base-64 digits each: '0' ('0' + 0)
-- to 'o' ('0' + 63), most significant first.
@pub(pkg) def alphanumeric =
  {}
",
        long_literal(&body, 96)
    )
}

// --- cases --------------------------------------------------------------------------

/// A small deterministic generator, so that the cases are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

const WORDS: &[&str] = &[
    "a",
    "an",
    "the",
    "quick",
    "brown",
    "fox",
    "jumps",
    "over",
    "lazy",
    "dog",
    "Lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet,",
    "consectetur",
    "adipiscing",
    "elit.",
    "extraordinarily",
    "antidisestablishmentarianism",
    "supercalifragilistic",
    "co-operate",
    "well-known",
    "--foo-bar",
    "foo--bar",
    "x-",
    "-y",
    "a-b-c-d-e-f",
    "state-of-the-art",
    "e.g.",
    "U.S.A.",
    "32.3",
    "1,000",
    "http://example.com/a-b",
    "日本語",
    "テキスト",
    "中文字符串",
    "한국어",
    "👍",
    "👨\u{200d}👩\u{200d}👧",
    "🇺🇸",
    "e\u{301}",
    "naïve",
    "café",
    "Ωμέγα",
    "שלום",
    "مرحبا",
    "ภาษาไทย",
    "re\u{ad}enter",
    "x\u{2014}y",
    "1\u{2013}2",
    "(parens)",
    "\"quoted\"",
    "it's",
    "\u{1b}[31mred\u{1b}[0m",
    "\u{1b}[1;4mbold\u{1b}[m",
    "\u{1b}]8;;http://x\u{1b}\\link\u{1b}]8;;\u{1b}\\",
    "\u{1b}]0;title\u{7}",
    "\u{1b}x",
    "\u{1b}",
    "tab\there",
    "nb\u{a0}sp",
    "",
    "#",
    "*",
    ">",
    "//",
    "+",
];

const SEPARATORS: &[&str] = &[
    " ", " ", " ", " ", "  ", "   ", "\n", "\n", "\r\n", "\n\n", " \n", "\t", "\u{3000}", "",
];

const PREFIXES: &[&str] = &[
    "", "", "", "  ", "    ", "\t", "* ", "> ", "- ", "# ", "// ", "  * ", "> > ", "\u{3000}",
];

/// Some text built from the pools above.
fn random_text(rng: &mut Rng) -> String {
    let n = rng.below(18);
    let mut s = String::new();
    if rng.chance(30) {
        s.push_str(rng.pick(PREFIXES));
    }
    for i in 0..n {
        if i > 0 {
            s.push_str(rng.pick(SEPARATORS));
            if rng.chance(15) {
                s.push_str(rng.pick(PREFIXES));
            }
        }
        s.push_str(rng.pick(WORDS));
    }
    if rng.chance(20) {
        s.push_str(rng.pick(SEPARATORS));
    }
    s
}

/// Text laid out in indented lines, for `dedent`, `unfill` and `refill`.
fn random_block(rng: &mut Rng) -> String {
    let prefix = rng.pick(PREFIXES);
    let lines = rng.below(6);
    let ending = if rng.chance(20) { "\r\n" } else { "\n" };
    let mut s = String::new();
    for i in 0..lines {
        if i > 0 || rng.chance(50) {
            s.push_str(if rng.chance(15) {
                rng.pick(PREFIXES)
            } else {
                prefix
            });
        }
        if rng.chance(20) {
            s.push_str(rng.pick(PREFIXES));
        }
        if !rng.chance(10) {
            s.push_str(&random_text(rng).replace(['\n', '\r'], " "));
        }
        if i + 1 < lines || rng.chance(50) {
            s.push_str(if rng.chance(10) { "\n" } else { ending });
        }
    }
    s
}

/// The settings a case uses, written as `Tests.mw` reads them.
#[derive(Clone, Copy)]
struct Setting {
    width: usize,
    crlf: bool,
    initial: &'static str,
    subsequent: &'static str,
    break_words: bool,
    /// 0 first fit, 1 optimal fit, 2 optimal fit with other penalties.
    algorithm: u8,
    unicode: bool,
    hyphens: bool,
}

const OTHER_PENALTIES: Penalties = Penalties {
    nline_penalty: 500,
    overflow_penalty: 100,
    short_last_line_fraction: 3,
    short_last_line_penalty: 10,
    hyphen_penalty: 50,
};

impl Setting {
    fn random(rng: &mut Rng) -> Setting {
        Setting {
            width: rng.pick(&[0, 1, 2, 3, 4, 5, 6, 8, 10, 12, 15, 20, 25, 30, 40, 60, 80]),
            crlf: rng.chance(15),
            initial: rng.pick(PREFIXES),
            subsequent: rng.pick(PREFIXES),
            break_words: rng.chance(70),
            algorithm: rng.pick(&[0, 1, 1, 2]),
            unicode: rng.chance(60),
            hyphens: rng.chance(70),
        }
    }

    fn options(&self) -> Options<'static> {
        Options::new(self.width)
            .line_ending(if self.crlf {
                LineEnding::CRLF
            } else {
                LineEnding::LF
            })
            .initial_indent(self.initial)
            .subsequent_indent(self.subsequent)
            .break_words(self.break_words)
            .wrap_algorithm(match self.algorithm {
                0 => WrapAlgorithm::FirstFit,
                1 => WrapAlgorithm::new_optimal_fit(),
                _ => WrapAlgorithm::OptimalFit(OTHER_PENALTIES),
            })
            .word_separator(if self.unicode {
                WordSeparator::UnicodeBreakProperties
            } else {
                WordSeparator::AsciiSpace
            })
            .word_splitter(if self.hyphens {
                WordSplitter::HyphenSplitter
            } else {
                WordSplitter::NoHyphenation
            })
    }

    fn write(&self, out: &mut String) {
        digits(out, self.width as u64, 2);
        flag(out, self.crlf);
        text(out, self.initial);
        text(out, self.subsequent);
        flag(out, self.break_words);
        digits(out, self.algorithm.into(), 1);
        flag(out, self.unicode);
        flag(out, self.hyphens);
    }
}

fn lines(out: &mut String, lines: &[impl AsRef<str>]) {
    digits(out, lines.len() as u64, 3);
    for l in lines {
        text(out, l.as_ref());
    }
}

/// `f()`, or `None` if it panics.
fn guard<T>(f: impl FnOnce() -> T + std::panic::UnwindSafe) -> Option<T> {
    std::panic::catch_unwind(f).ok()
}

fn cases() -> String {
    let mut rng = Rng(0x7e47_7a4a_c0ff_ee42);
    let mut body = String::new();
    let mut counts = [0usize; 8];

    // Kind 0: wrap and fill.
    for _ in 0..3000 {
        let t = random_text(&mut rng);
        let setting = Setting::random(&mut rng);
        let Some((wrapped, filled)) = guard(|| {
            (
                textwrap::wrap(&t, setting.options()),
                textwrap::fill(&t, setting.options()),
            )
        }) else {
            continue;
        };
        counts[0] += 1;
        digits(&mut body, 0, 1);
        text(&mut body, &t);
        setting.write(&mut body);
        lines(&mut body, &wrapped);
        text(&mut body, &filled);
    }

    // Kind 1: fill_inplace, which wraps at ASCII spaces only.
    for _ in 0..600 {
        let t = random_text(&mut rng);
        let width = rng.pick(&[0usize, 1, 3, 5, 10, 20, 40]);
        let Some(filled) = guard(|| {
            let mut s = t.clone();
            textwrap::fill_inplace(&mut s, width);
            s
        }) else {
            continue;
        };
        counts[1] += 1;
        digits(&mut body, 1, 1);
        text(&mut body, &t);
        digits(&mut body, width as u64, 2);
        text(&mut body, &filled);
    }

    // Kind 2: indent.
    for _ in 0..600 {
        let t = if rng.chance(50) {
            random_block(&mut rng)
        } else {
            random_text(&mut rng)
        };
        let prefix = rng.pick(PREFIXES);
        counts[2] += 1;
        digits(&mut body, 2, 1);
        text(&mut body, &t);
        text(&mut body, prefix);
        text(&mut body, &textwrap::indent(&t, prefix));
    }

    // Kind 3: dedent.
    for _ in 0..800 {
        let t = if rng.chance(80) {
            random_block(&mut rng)
        } else {
            random_text(&mut rng)
        };
        counts[3] += 1;
        digits(&mut body, 3, 1);
        text(&mut body, &t);
        text(&mut body, &textwrap::dedent(&t));
    }

    // Kind 4: unfill, of filled text and of indented blocks.
    for _ in 0..800 {
        let t = if rng.chance(70) {
            let setting = Setting {
                width: rng.pick(&[5, 10, 20, 30]),
                ..Setting::random(&mut rng)
            };
            let source = random_text(&mut rng);
            let Some(f) = guard(|| textwrap::fill(&source, setting.options())) else {
                continue;
            };
            f
        } else {
            random_block(&mut rng)
        };
        let Some((unfilled, found)) = guard(|| {
            let (u, o) = textwrap::unfill(&t);
            let found = (
                o.width,
                o.initial_indent.len(),
                o.subsequent_indent.len(),
                o.line_ending == LineEnding::CRLF,
            );
            (u, found)
        }) else {
            continue;
        };
        counts[4] += 1;
        digits(&mut body, 4, 1);
        text(&mut body, &t);
        text(&mut body, &unfilled);
        digits(&mut body, found.0 as u64, 3);
        digits(&mut body, found.1 as u64, 2);
        digits(&mut body, found.2 as u64, 2);
        flag(&mut body, found.3);
    }

    // Kind 5: refill.
    for _ in 0..800 {
        let t = random_block(&mut rng);
        let setting = Setting::random(&mut rng);
        let Some(refilled) = guard(|| textwrap::refill(&t, setting.options())) else {
            continue;
        };
        counts[5] += 1;
        digits(&mut body, 5, 1);
        text(&mut body, &t);
        setting.write(&mut body);
        text(&mut body, &refilled);
    }

    // Kind 6: wrap_columns.
    for _ in 0..600 {
        let t = random_text(&mut rng);
        let setting = Setting::random(&mut rng);
        let columns = 1 + rng.below(4);
        let gaps = [
            rng.pick(&["", "|", "| "]),
            rng.pick(&["", " ", " | "]),
            rng.pick(&["", "|", " |"]),
        ];
        let Some(result) = guard(|| {
            textwrap::wrap_columns(&t, columns, setting.options(), gaps[0], gaps[1], gaps[2])
        }) else {
            continue;
        };
        counts[6] += 1;
        digits(&mut body, 6, 1);
        text(&mut body, &t);
        setting.write(&mut body);
        digits(&mut body, columns as u64, 1);
        for g in gaps {
            text(&mut body, g);
        }
        lines(&mut body, &result);
    }

    // Kind 7: display_width.
    for _ in 0..400 {
        let t = random_text(&mut rng);
        counts[7] += 1;
        digits(&mut body, 7, 1);
        text(&mut body, &t);
        digits(&mut body, textwrap::core::display_width(&t) as u64, 3);
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "-- GENERATED by scripts/generate.sh from textwrap {UPSTREAM_VERSION}.
-- Do not edit: run the script again instead.
--
-- Inputs, with what textwrap makes of them, for `Tests.mw`: {} wrap and fill,
-- {} fill_inplace, {} indent, {} dedent, {} unfill, {} refill, {} wrap_columns
-- and {} display_width cases.
--
{HEADER}

-- Each case starts with its kind (1 base-64 digit), and its fields follow in
-- the order `Tests.mw` reads them. A string is its length in bytes (3 digits)
-- and then its bytes; a list of strings is a count (3) and the strings; a
-- setting is the width (2), CRLF (1), the two indents, break words (1), the
-- algorithm (1: first fit, optimal, optimal with other penalties), Unicode
-- word separation (1) and hyphen splitting (1).
@cfg(test)
@pub(pkg) def cases =
  {}",
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        counts[4],
        counts[5],
        counts[6],
        counts[7],
        long_literal(&body, 96)
    );
    out
}
