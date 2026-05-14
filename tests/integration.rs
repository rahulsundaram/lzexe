/// Integration tests using MIT-licensed fixtures from the official LZEXE 0.91
/// distribution (https://bellard.org/lzexe/lzexe91.zip).
///
/// Both LZEXE.EXE and UPACKEXE.EXE were shipped by Fabrice Bellard self-
/// compressed with LZEXE 0.91, making them ideal self-referential fixtures
/// that carry no copyright concerns.
use std::path::PathBuf;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("could not read fixture {name}: {e}"))
}

// ── detection ─────────────────────────────────────────────────────────────────

#[test]
fn detects_lzexe_exe_as_compressed() {
    let data = fixture("LZEXE.EXE");
    assert!(lzexe::is_compressed(&data));
}

#[test]
fn detects_upackexe_as_compressed() {
    let data = fixture("UPACKEXE.EXE");
    assert!(lzexe::is_compressed(&data));
}

#[test]
fn does_not_flag_random_bytes_as_compressed() {
    assert!(!lzexe::is_compressed(b"hello world"));
}

#[test]
fn does_not_flag_bare_mz_header_as_compressed() {
    // Valid MZ header with no LZEXE markers
    let mut hdr = vec![0u8; 64];
    hdr[0] = 0x4D; hdr[1] = 0x5A; // MZ
    hdr[6] = 2;                    // 2 relocation entries (not 0)
    hdr[0x18] = 0x40;              // relocation table not at 0x1C
    assert!(!lzexe::is_compressed(&hdr));
}

// ── passthrough ───────────────────────────────────────────────────────────────

#[test]
fn passthrough_returns_input_unchanged_for_non_exe() {
    let input = b"not an exe at all".to_vec();
    let out = lzexe::decompress(&input).unwrap();
    assert_eq!(out, input);
}

#[test]
fn passthrough_returns_input_unchanged_for_plain_mz() {
    // Minimal valid MZ that is NOT LZEXE-packed
    let mut exe = vec![0u8; 512];
    exe[0] = 0x4D; exe[1] = 0x5A;  // MZ
    exe[4] = 2;                     // 2 header paragraphs
    exe[6] = 1;                     // 1 reloc entry (not 0, so not LZEXE)
    let out = lzexe::decompress(&exe).unwrap();
    assert_eq!(out, exe);
}

// ── decompression correctness ─────────────────────────────────────────────────

/// Decompresses LZEXE.EXE and checks output size (regression guard).
///
/// Note: the size is independently verifiable — the Pascal source compiles
/// to a known code+data layout with Turbo Pascal 5.0, and 19440 bytes is
/// the expected decompressed size of the 1990 binary.
#[test]
fn decompresses_lzexe_exe_to_expected_size() {
    let data = fixture("LZEXE.EXE");
    let out = lzexe::decompress(&data).expect("decompression failed");
    assert_eq!(out.len(), 19440, "unexpected decompressed size");
}

/// Decompresses UPACKEXE.EXE and checks output size (regression guard).
#[test]
fn decompresses_upackexe_to_expected_size() {
    let data = fixture("UPACKEXE.EXE");
    let out = lzexe::decompress(&data).expect("decompression failed");
    assert_eq!(out.len(), 11840, "unexpected decompressed size");
}

/// Checks that the decompressed LZEXE.EXE contains the version banner that
/// appears verbatim in lzexe.pas: writeln('LZEXE.EXE  Version 0.91 ...')
/// This is an independent correctness check derived from the Pascal source.
#[test]
fn decompressed_lzexe_exe_contains_version_string() {
    let data = fixture("LZEXE.EXE");
    let out = lzexe::decompress(&data).expect("decompression failed");
    let needle = b"LZEXE.EXE  Version 0.91";
    assert!(
        out.windows(needle.len()).any(|w| w == needle),
        "version string not found in decompressed LZEXE.EXE"
    );
}

/// Checks that the decompressed UPACKEXE.EXE contains its known banner string
/// from the Pascal source: writeln('UPACKEXE Version 1.00 ...')
#[test]
fn decompressed_upackexe_contains_version_string() {
    let data = fixture("UPACKEXE.EXE");
    let out = lzexe::decompress(&data).expect("decompression failed");
    let needle = b"UPACKEXE";
    assert!(
        out.windows(needle.len()).any(|w| w == needle),
        "version string not found in decompressed UPACKEXE.EXE"
    );
}

/// Decompressed output must start with MZ and have a valid relocation table.
#[test]
fn decompressed_lzexe_exe_is_valid_mz() {
    let data = fixture("LZEXE.EXE");
    let out = lzexe::decompress(&data).unwrap();
    assert_eq!(&out[0..2], b"MZ", "output does not start with MZ");
    // Must have relocation entries (the uncompressed LZEXE.EXE has many)
    let reloc_count = u16::from_le_bytes([out[6], out[7]]);
    assert!(reloc_count > 0, "expected relocation entries in decompressed output");
    // Must NOT look like an LZEXE-packed file (reloc count would be 0)
    assert!(!lzexe::is_compressed(&out), "output should not itself be LZEXE-packed");
}

/// Calling decompress on already-decompressed output is idempotent.
#[test]
fn decompress_is_idempotent_on_plain_mz() {
    let data = fixture("LZEXE.EXE");
    let once = lzexe::decompress(&data).unwrap();
    let twice = lzexe::decompress(&once).unwrap();
    assert_eq!(once, twice);
}

// ── golden file tests ─────────────────────────────────────────────────────────

#[test]
fn lzexe_exe_matches_golden() {
    let compressed = fixture("LZEXE.EXE");
    let golden     = fixture("LZEXE_GOLD.EXE");
    let out = lzexe::decompress(&compressed).expect("decompression failed");
    assert_eq!(out, golden, "LZEXE.EXE decompressed output differs from golden");
}

#[test]
fn upackexe_matches_golden() {
    let compressed = fixture("UPACKEXE.EXE");
    let golden     = fixture("UPACKEXE_GOLD.EXE");
    let out = lzexe::decompress(&compressed).expect("decompression failed");
    assert_eq!(out, golden, "UPACKEXE.EXE decompressed output differs from golden");
}

// ── gamecompjs cross-version fixtures (GPL-3.0, camoto-project/gamecompjs) ────
// Synthetic EXE test vectors covering all three LZEXE versions.
// Source: https://github.com/camoto-project/gamecompjs/tree/46a612c/test/cmp-lzexe

#[test]
fn gamecompjs_lzexe91_matches_golden() {
    let out = lzexe::decompress(&fixture("lzexe91.bin")).expect("decompression failed");
    assert_eq!(out, fixture("gamecompjs_gold.bin"));
}

#[test]
fn gamecompjs_lzexe91e_matches_golden() {
    // 0.91e has an extra 0x50 prefix byte in the stub; output is identical to 0.91
    let out = lzexe::decompress(&fixture("lzexe91e.bin")).expect("decompression failed");
    assert_eq!(out, fixture("gamecompjs_gold.bin"));
}

#[test]
fn gamecompjs_lzexe90_payload_matches() {
    // 0.90 has a smaller decompressor stub so minalloc differs, but the load
    // image must be byte-for-byte identical to the 0.91 output.
    let out90 = lzexe::decompress(&fixture("lzexe90.bin")).expect("decompression failed");
    let out91 = fixture("gamecompjs_gold.bin");
    let header_paras_90 = u16::from_le_bytes([out90[8], out90[9]]) as usize;
    let header_paras_91 = u16::from_le_bytes([out91[8], out91[9]]) as usize;
    assert_eq!(out90[header_paras_90 * 16..], out91[header_paras_91 * 16..]);
}
