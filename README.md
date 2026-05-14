# lzexe

A pure-Rust decompressor for DOS EXE files packed with
[LZEXE](https://bellard.org/lzexe/) — the packer written by Fabrice Bellard
in 1989/1990 that shipped with many early DOS games (id Software, Apogee,
Softdisk, and others). LZEXE uses the **LZSS** (Lempel-Ziv-Storer-Szymanski)
algorithm internally; this crate is unrelated to LZW.

Supports LZEXE **0.90**, **0.91**, and **0.91e**.

## Library usage

```rust
let compressed = std::fs::read("program.exe").unwrap();

// Transparently handles both compressed and uncompressed EXEs.
// Returns Ok(input.to_vec()) for uncompressed files — no allocation
// until the LZEXE signature is confirmed.
let data = lzexe::decompress(&compressed).unwrap();
```

Check without decompressing:

```rust
if lzexe::is_compressed(&bytes) {
    println!("Packed with LZEXE");
}
```

## `unlzexe` CLI

A drop-in replacement for the classic DOS `UNLZEXE` utility:

```
unlzexe input.exe [output.exe]
```

If `output.exe` is omitted the decompressed file is written to the same
directory as the input with the same name. Exit codes: `0` = success,
`1` = error, `2` = input is not LZEXE-compressed.

Install from crates.io:

```
cargo install lzexe
```

## Output format

The returned `Vec<u8>` is a valid MZ EXE with a reconstructed header followed
by the decompressed load image. Absolute file offsets in the output match those
of the original uncompressed binary, so asset extractors that rely on fixed
offsets work without modification.

All EXE header fields are recovered from the packed file's info block and
the packer's header transformation formulas (see [lzexe.pas][src]):

| Field | Recovery method |
|---|---|
| IP, CS, SP, SS | Stored verbatim in the LZEXE info block at `CS:0` |
| minalloc | `compressed_minalloc − (decalage + dcmpsizepar + 9)` |
| maxalloc | Same subtraction; `0xFFFF` if the compressed value was clamped |
| Relocation table | Delta-decoded from the packed reloc stream at `CS:0x158` (v0.91) or `CS:0x19D` (v0.90) |

[src]: https://bellard.org/lzexe/lzexe91-src.zip

## No dependencies

Zero runtime dependencies. `no_std`-compatible (requires `alloc`).

## Testing

The integration tests use the official LZEXE 0.91 binaries (`LZEXE.EXE` and
`UPACKEXE.EXE`) as fixtures — both are self-compressed with LZEXE 0.91 and
distributed by Bellard under the MIT license. Golden output files are committed
alongside them for byte-exact regression testing.

Additional synthetic test vectors from
[camoto-project/gamecompjs](https://github.com/camoto-project/gamecompjs)
(GPL-3.0) cover all three LZEXE versions: 0.90, 0.91, and 0.91e.

```
cargo test
```

Correctness is verified independently of our own output by checking that the
decompressed binaries contain version strings that appear verbatim in the
[Pascal source][src] (`"LZEXE.EXE  Version 0.91"`, `"UPACKEXE"`).

## References

- [LZEXE homepage](https://bellard.org/lzexe/) — Fabrice Bellard's original tool and source release (MIT license, May 2025)
- [LZEXE 0.91 source (zip)](https://bellard.org/lzexe/lzexe91-src.zip) — official Pascal + ASM source; primary reference for this implementation
- [LZEXE decompression internals](https://cosmodoc.org/topics/lzexe/) — step-by-step breakdown of the decompressor by Scott Smitelli
- [Modding Wiki: LZEXE](https://moddingwiki.shikadi.net/wiki/LZEXE) — version signatures and variant notes
- [gamecompjs cmp-lzexe.js](https://github.com/Malvineous/gamecompjs/blob/master/formats/cmp-lzexe.js) — JavaScript reference implementation by Adam Nielsen (GPL-3.0); referenced for relocation table decoding
- [UNLZEXE](https://www.shikadi.net/moddingwiki/UNLZEXE) — the classic DOS-era decompressor utility
