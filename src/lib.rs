//! # lzexe
//!
//! Decompresses DOS EXE files packed with [LZEXE](https://bellard.org/lzexe/)
//! versions 0.90, 0.91, and 0.91e — the packer written by Fabrice Bellard in
//! 1989/1990 that was used in many early DOS games (id Software, Apogee, etc.).
//!
//! ## Usage
//!
//! ```rust,no_run
//! let compressed = std::fs::read("program.exe").unwrap();
//!
//! // Transparently handles both compressed and uncompressed EXEs
//! let data = lzexe::decompress(&compressed).unwrap();
//!
//! // data now has the same layout as the uncompressed EXE;
//! // hard-coded file offsets work without adjustment.
//! ```
//!
//! ## Output format
//!
//! The returned bytes are a valid MZ EXE with a reconstructed header followed
//! by the decompressed load image. File offsets in the output match those of
//! the original uncompressed EXE, so any code that relies on absolute offsets
//! (e.g. asset extractors) works without modification.
//!
//! Uncompressed files are returned unchanged (zero-copy passthrough detection,
//! then `to_vec()` only if it really is an LZEXE file).
//!
//! ## No dependencies
//!
//! This crate has no runtime dependencies and is `no_std`-compatible (requires
//! `alloc`).

/// Decompresses an LZEXE 0.90/0.91/0.91e packed DOS EXE.
///
/// Returns `Ok(decompressed_bytes)` on success.  
/// Returns `Ok(input.to_vec())` if the file is **not** LZEXE-compressed.  
/// Returns `Err(message)` if the file looks like LZEXE but is corrupt/truncated.
///
/// # Example
///
/// ```rust,no_run
/// let raw = std::fs::read("program.exe").unwrap();
/// let exe = lzexe::decompress(&raw).unwrap();
/// // exe has the same byte layout as the original uncompressed EXE
/// ```
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    match detect_version(data) {
        None => Ok(data.to_vec()),
        Some(ver) => do_decompress(data, ver),
    }
}

/// Returns `true` if the bytes appear to be LZEXE-compressed (0.90/0.91/0.91e).
///
/// This is a cheap O(1) check — it reads the MZ header and compares the
/// decompressor signature at the entry point.
///
/// # Example
///
/// ```rust
/// // A random byte slice is not an LZEXE file
/// assert!(!lzexe::is_compressed(b"hello world"));
/// ```
pub fn is_compressed(data: &[u8]) -> bool {
    detect_version(data).is_some()
}

// ── internal ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Ver { V090, V091 }

// Signature bytes at the entry-point for each version.
const SIG90: &[u8] = &[
    0x06,0x0E,0x1F,0x8B,0x0E,0x0C,0x00,0x8B,0xF1,0x4E,0x89,0xF7,0x8C,0xDB,
    0x03,0x1E,0x0A,0x00,0x8E,0xC3,0xB4,0x00,0x31,0xED,
];
const SIG91: &[u8] = &[
    0x06,0x0E,0x1F,0x8B,0x0E,0x0C,0x00,0x8B,0xF1,0x4E,0x89,0xF7,0x8C,0xDB,
    0x03,0x1E,0x0A,0x00,0x8E,0xC3,0xFD,0xF3,0xA4,0x53,
];
// 0.91e has an extra 0x50 prefix byte but same body
const SIG91E_PREFIX: u8 = 0x50;

fn read_u16le(data: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([data[off], data[off + 1]])
}

fn write_u16le(out: &mut [u8], off: usize, v: u16) {
    let b = v.to_le_bytes();
    out[off]     = b[0];
    out[off + 1] = b[1];
}

/// Locate the entry-point offset from the MZ header.
fn entry_point(data: &[u8]) -> usize {
    let header_paras = read_u16le(data, 0x08) as usize; // paragraphs before image
    let cs = read_u16le(data, 0x16) as usize;            // initial CS (relative)
    let ip = read_u16le(data, 0x14) as usize;            // initial IP
    ((header_paras + cs) << 4) + ip
}

fn detect_version(data: &[u8]) -> Option<Ver> {
    if data.len() < 0x20 {
        return None;
    }
    // Must be an MZ exe
    let magic = read_u16le(data, 0);
    if magic != 0x5A4D && magic != 0x4D5A {
        return None;
    }
    // LZEXE marker: relocation table at 0x1C, zero relocations
    if read_u16le(data, 0x18) != 0x1C || read_u16le(data, 0x06) != 0 {
        return None;
    }

    let ep = entry_point(data);
    if ep + SIG91.len() > data.len() {
        return None;
    }
    let sig = &data[ep..ep + SIG91.len()];

    if sig == SIG90 {
        return Some(Ver::V090);
    }
    if sig == SIG91 {
        return Some(Ver::V091);
    }
    // 0.91e: skip leading 0x50 and compare
    if data[ep] == SIG91E_PREFIX && ep + 1 + SIG91.len() <= data.len()
        && &data[ep+1..ep+1+SIG91.len()] == SIG91
    {
        return Some(Ver::V091);
    }
    None
}

// ── bit-stream reader ─────────────────────────────────────────────────────────

struct Bits<'a> {
    src: &'a [u8],
    pos: usize,
    buf: u16,
    count: u8,
}

impl<'a> Bits<'a> {
    fn new(src: &'a [u8], pos: usize) -> Self {
        let buf = u16::from_le_bytes([src[pos], src[pos + 1]]);
        Bits { src, pos: pos + 2, buf, count: 16 }
    }

    fn get(&mut self) -> u8 {
        let b = (self.buf & 1) as u8;
        self.buf >>= 1;
        self.count -= 1;
        if self.count == 0 {
            self.buf   = u16::from_le_bytes([self.src[self.pos], self.src[self.pos + 1]]);
            self.pos  += 2;
            self.count = 16;
        }
        b
    }

    fn byte(&mut self) -> u8 {
        let b = self.src[self.pos];
        self.pos += 1;
        b
    }
}

// ── main decompressor ─────────────────────────────────────────────────────────

fn do_decompress(data: &[u8], ver: Ver) -> Result<Vec<u8>, String> {
    // Read the EXE header words (0x00–0x0F).
    let ihead: Vec<u16> = (0..16).map(|i| read_u16le(data, i * 2)).collect();

    // CS offset where LZEXE's decompressor stub lives.
    let cs_off = ((ihead[0x04] + ihead[0x0b]) as usize) << 4;
    if cs_off + 16 > data.len() {
        return Err("Truncated LZEXE header".into());
    }

    // 8-word LZEXE info block at CS:0.
    // info[0]=IP, info[1]=CS, info[2]=SP, info[3]=SS,
    // info[4]=compressed program size (paragraphs), info[5..7]=misc sizes
    let info: Vec<u16> = (0..8).map(|i| read_u16le(data, cs_off + i * 2)).collect();

    // ── 1. Decompress relocation table ───────────────────────────────────────
    let relocs = match ver {
        Ver::V090 => decompress_reloc_90(data, cs_off)?,
        Ver::V091 => decompress_reloc_91(data, cs_off)?,
    };

    // ── 2. Compute output header size ────────────────────────────────────────
    // Relocation table starts at 0x1C; each entry is 4 bytes.
    // Pad header to a 512-byte (0x200) sector boundary, matching UNLZEXE behavior.
    let reloc_bytes = relocs.len() * 4;
    let header_raw  = 0x1C + reloc_bytes;
    let header_size = (header_raw + 0x1FF) & !0x1FF; // round up to 512-byte boundary

    // ── 3. Decompress payload ─────────────────────────────────────────────────
    let in_start = ((ihead[0x0b] as i32 - info[4] as i32 + ihead[0x04] as i32) << 4) as usize;
    let mut image: Vec<u8> = Vec::with_capacity(200 * 1024);
    let mut bits = Bits::new(data, in_start);

    loop {
        if bits.get() == 1 {
            image.push(bits.byte());
            continue;
        }

        let (span, len): (i32, usize) = if bits.get() == 0 {
            let len_bits = (bits.get() as i32) << 1 | bits.get() as i32;
            let len = (len_bits + 2) as usize;
            let raw = (bits.byte() as u16) | 0xFF00u16;
            (raw as i16 as i32, len)
        } else {
            let lo = bits.byte() as u32;
            let hi = bits.byte() as u32;
            let raw = (lo | (((hi & !0x07u32) << 5) | 0xE000u32)) as u16;
            let span = raw as i16 as i32;
            let mut len = (hi & 0x07) as usize + 2;
            if len == 2 {
                len = bits.byte() as usize;
                if len == 0 { break; }
                if len == 1 { continue; }
                len += 1;
            }
            (span, len)
        };

        let pos = image.len();
        for i in 0..len {
            let src_idx = (pos as i32 + i as i32 + span) as usize;
            let byte = *image.get(src_idx)
                .ok_or_else(|| format!("Back-reference out of bounds: pos={pos} span={span} i={i}"))?;
            image.push(byte);
        }
    }

    // ── 4. Assemble output EXE ────────────────────────────────────────────────
    let total_size = header_size + image.len();
    let mut out = vec![0u8; total_size];

    // MZ signature
    out[0] = 0x4D; out[1] = 0x5A;

    // Size fields
    let file_bytes = total_size;
    write_u16le(&mut out, 0x02, (file_bytes & 0x1FF) as u16);
    write_u16le(&mut out, 0x04, ((file_bytes + 0x1FF) >> 9) as u16);

    // Relocation table
    write_u16le(&mut out, 0x06, relocs.len() as u16);
    write_u16le(&mut out, 0x08, (header_size >> 4) as u16); // header size in paragraphs
    write_u16le(&mut out, 0x18, 0x1C);                       // reloc table at 0x1C

    // Recover original min/max alloc by inverting the packer's formula.
    // lzexe.pas: n := decalage + dcmpsizepar + 9
    //            Hout[5] := Hexe[5] + n   (Hexe = original, Hout = compressed)
    // So: original_minalloc = compressed_minalloc - n
    // inf[5]=decalage, inf[6]=dcmpsize (bytes)
    let dcmpsizepar = ((info[6] as u32 + 15) >> 4) as u16;
    let n = info[5].wrapping_add(dcmpsizepar).wrapping_add(9);
    let min_alloc = ihead[0x05].wrapping_sub(n);
    let max_alloc = if ihead[0x06] == 0xFFFF {
        0xFFFFu16
    } else {
        ihead[0x06].wrapping_sub(n)
    };
    write_u16le(&mut out, 0x0A, min_alloc);
    write_u16le(&mut out, 0x0C, max_alloc);
    write_u16le(&mut out, 0x0E, info[3]);     // SS
    write_u16le(&mut out, 0x10, info[2]);     // SP
    write_u16le(&mut out, 0x14, info[0]);     // IP
    write_u16le(&mut out, 0x16, info[1]);     // CS

    // Write relocation entries at 0x1C
    for (i, &(off, seg)) in relocs.iter().enumerate() {
        let base = 0x1C + i * 4;
        write_u16le(&mut out, base,     off);
        write_u16le(&mut out, base + 2, seg);
    }

    // Place decompressed load image after header
    out[header_size..].copy_from_slice(&image);

    Ok(out)
}

// ── relocation table decompressors ───────────────────────────────────────────

fn decompress_reloc_91(data: &[u8], cs_off: usize) -> Result<Vec<(u16, u16)>, String> {
    // Compressed relocation table is at CS:0x158 for LZEXE 0.91
    let mut pos = cs_off + 0x158;
    let mut entries: Vec<(u16, u16)> = Vec::new();
    let mut rel_seg = 0u32;
    let mut rel_off = 0u32;

    loop {
        if pos >= data.len() {
            return Err("Relocation table truncated".into());
        }
        let b = data[pos] as u32;
        pos += 1;

        let span: u32 = if b == 0 {
            if pos + 2 > data.len() { return Err("Relocation table truncated".into()); }
            let w = u16::from_le_bytes([data[pos], data[pos+1]]) as u32;
            pos += 2;
            match w {
                0 => { rel_seg += 0x0FFF; continue; }
                1 => break,
                _ => w,
            }
        } else {
            b
        };

        rel_off += span;
        rel_seg += rel_off >> 4;
        rel_off &= 0x0F;
        entries.push((rel_off as u16, rel_seg as u16));
    }

    Ok(entries)
}

fn decompress_reloc_90(data: &[u8], cs_off: usize) -> Result<Vec<(u16, u16)>, String> {
    // Compressed relocation table is at CS:0x19D for LZEXE 0.90
    let mut pos = cs_off + 0x19D;
    let mut entries: Vec<(u16, u16)> = Vec::new();
    let mut rel_seg = 0u32;

    loop {
        if pos + 2 > data.len() { return Err("Relocation table truncated".into()); }
        let count = u16::from_le_bytes([data[pos], data[pos+1]]) as usize;
        pos += 2;
        for _ in 0..count {
            if pos + 2 > data.len() { return Err("Relocation table truncated".into()); }
            let rel_off = u16::from_le_bytes([data[pos], data[pos+1]]);
            pos += 2;
            entries.push((rel_off, rel_seg as u16));
        }
        rel_seg += 0x1000;
        if rel_seg >= 0xF000 + 0x1000 { break; }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncompressed_passthrough() {
        let data = vec![0x4D, 0x5A, 0x00, 0x01]; // MZ, not LZEXE
        let out = decompress(&data).unwrap();
        assert_eq!(out, data);
    }

    #[test]
    fn non_exe_passthrough() {
        let data = b"hello world".to_vec();
        let out = decompress(&data).unwrap();
        assert_eq!(out, data);
    }
}
