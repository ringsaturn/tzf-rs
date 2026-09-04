//! Reader for the TZF embedded binary format (`.tzb`).
//!
//! Implements the container and E profile, plus the FUZZY section,
//! mirroring the Go reference implementation in
//! `github.com/ringsaturn/tzf/v2/internal/embedbin`.

mod expand;
mod fuzzy;
mod raycast;
mod reader;
mod tile;

pub(crate) use expand::ExpandedPolygon;
pub(crate) use raycast::raycast_seg;
pub(crate) use reader::Reader;
pub(crate) use tile::TileId;

use geometry_rs::I32Point;
use std::fmt;

pub(crate) const HEADER_SIZE: u64 = 64;
pub(crate) const SECTION_ENTRY_LEN: u64 = 16;
pub(crate) const FOOTER_SIZE: u64 = 4;
pub(crate) const FORMAT_MAJOR: u8 = 1;
pub(crate) const COORD_SCALE: u32 = 100_000;

/// Header byte assigned as `profile` in format revision 1.1.
pub(crate) const PROFILE_OFFSET: usize = 48;
pub(crate) const PROFILE_E: u8 = 0;
pub(crate) const PROFILE_M: u8 = 1;

pub(crate) const FLAG_GRID: u32 = 1 << 0;
pub(crate) const FLAG_NO_SHORTCUT: u32 = 1 << 1;

pub(crate) const SECTION_NAMES: u32 = 1;
pub(crate) const SECTION_TZDIR: u32 = 2;
pub(crate) const SECTION_POLYDIR: u32 = 3;
pub(crate) const SECTION_RINGDIR: u32 = 4;
pub(crate) const SECTION_RINGOPS: u32 = 5;
pub(crate) const SECTION_GROUPDIR: u32 = 6;
pub(crate) const SECTION_CHUNKDIR: u32 = 7;
pub(crate) const SECTION_GRID: u32 = 8;
pub(crate) const SECTION_POINTS: u32 = 9;
pub(crate) const SECTION_FUZZY: u32 = 10;
pub(crate) const SECTION_FLAT_POINTS: u32 = 12;
pub(crate) const SECTION_FLAT_RING_DIR: u32 = 13;
pub(crate) const SECTION_YSTRIPES: u32 = 14;

/// Sizes the per-type section table (types 1..14).
pub(crate) const SECTION_SLOTS: usize = 15;

pub(crate) const TZ_RECORD_LEN: u64 = 24;
pub(crate) const POLY_RECORD_LEN: u64 = 24;
pub(crate) const RING_RECORD_LEN: u64 = 28;
pub(crate) const GROUP_RECORD_LEN: u64 = 44;
pub(crate) const CHUNK_RECORD_LEN: u64 = 24;

pub(crate) const FUZZY_HEADER_LEN: u64 = 16;
/// Marks a FUZZY value word as a multi_dir group reference; the low 15 bits
/// are then a group index instead of a NAMES index.
pub(crate) const FUZZY_MULTI: u16 = 1 << 15;

/// Errors reported by the `.tzb` reader.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The file violates the format's structural rules.
    Malformed(&'static str),
    /// The file is a memory-image (`.tzm`, M profile), which tzf-rs does not
    /// consume: it exists for the Go runtime's zero-copy ring aliasing,
    /// which gains nothing here. Use the `.tzb` file.
    Profile,
    /// The file carries no FUZZY section.
    NoFuzzy,
    /// A timezone index is out of range.
    Index,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Malformed(what) => write!(f, "tzb: malformed file: {what}"),
            Error::Profile => {
                write!(
                    f,
                    "tzb: memory-image (.tzm) files are not supported; use .tzb"
                )
            }
            Error::NoFuzzy => write!(f, "tzb: file has no FUZZY section"),
            Error::Index => write!(f, "tzb: timezone index out of range"),
        }
    }
}

impl std::error::Error for Error {}

pub(crate) fn malformed<T>(what: &'static str) -> Result<T, Error> {
    Err(Error::Malformed(what))
}

/// Scaled-int32 bounding box, compared in `f64` like the Go reader.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BBox {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl BBox {
    pub(crate) fn read(raw: &[u8], off: usize) -> Self {
        Self {
            min_x: i32_le(raw, off),
            min_y: i32_le(raw, off + 4),
            max_x: i32_le(raw, off + 8),
            max_y: i32_le(raw, off + 12),
        }
    }

    fn ordered(self) -> bool {
        self.min_x <= self.max_x && self.min_y <= self.max_y
    }

    /// Ordered with all bounds in the storage domain (±180°/±90° scaled).
    pub(crate) fn in_domain(self) -> bool {
        self.ordered()
            && self.min_x >= -18_000_000
            && self.max_x <= 18_000_000
            && self.min_y >= -9_000_000
            && self.max_y <= 9_000_000
    }

    pub(crate) fn contains(self, x: f64, y: f64) -> bool {
        x >= f64::from(self.min_x)
            && x <= f64::from(self.max_x)
            && y >= f64::from(self.min_y)
            && y <= f64::from(self.max_y)
    }

    /// Whether a leftward ray from (x, y) can interact with segments inside
    /// this box: the raycast counts crossings at `lng >= x` only.
    pub(crate) fn ray_relevant(self, x: f64, y: f64) -> bool {
        y >= f64::from(self.min_y) && y <= f64::from(self.max_y) && f64::from(self.max_x) >= x
    }
}

pub(crate) fn point_in_domain(p: I32Point) -> bool {
    (-18_000_000..=18_000_000).contains(&p.x) && (-9_000_000..=9_000_000).contains(&p.y)
}

pub(crate) fn same_point(a: I32Point, b: I32Point) -> bool {
    a.x == b.x && a.y == b.y
}

pub(crate) fn align4(n: u64) -> u64 {
    (n + 3) & !3
}

// Little-endian field loads. Callers guarantee in-bounds offsets; the slice
// indexing still panics rather than reads out of bounds if they do not.
pub(crate) fn u16_le(raw: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([raw[off], raw[off + 1]])
}

pub(crate) fn u32_le(raw: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([raw[off], raw[off + 1], raw[off + 2], raw[off + 3]])
}

pub(crate) fn u64_le(raw: &[u8], off: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&raw[off..off + 8]);
    u64::from_le_bytes(b)
}

pub(crate) fn i16_le(raw: &[u8], off: usize) -> i16 {
    u16_le(raw, off) as i16
}

pub(crate) fn i32_le(raw: &[u8], off: usize) -> i32 {
    u32_le(raw, off) as i32
}

/// Adds a decoded delta to the previous coordinate, rejecting i32 overflow.
pub(crate) fn add_delta(prev: i32, delta: i32) -> Result<i32, Error> {
    match prev.checked_add(delta) {
        Some(v) => Ok(v),
        None => malformed("coordinate overflow"),
    }
}

/// CRC32 (IEEE 802.3), the polynomial `hash/crc32.IEEE` uses in Go.
/// Slicing-by-8: the checksum runs once over the whole file at open, so its
/// throughput dominates `EmbeddedFinder` open time.
pub(crate) fn crc32_ieee(data: &[u8]) -> u32 {
    const TABLES: [[u32; 256]; 8] = crc32_tables();
    let mut crc = !0u32;
    let mut chunks = data.chunks_exact(8);
    for chunk in &mut chunks {
        let lo = u32_le(chunk, 0) ^ crc;
        let hi = u32_le(chunk, 4);
        crc = TABLES[7][(lo & 0xff) as usize]
            ^ TABLES[6][((lo >> 8) & 0xff) as usize]
            ^ TABLES[5][((lo >> 16) & 0xff) as usize]
            ^ TABLES[4][(lo >> 24) as usize]
            ^ TABLES[3][(hi & 0xff) as usize]
            ^ TABLES[2][((hi >> 8) & 0xff) as usize]
            ^ TABLES[1][((hi >> 16) & 0xff) as usize]
            ^ TABLES[0][(hi >> 24) as usize];
    }
    for &b in chunks.remainder() {
        crc = TABLES[0][((crc ^ u32::from(b)) & 0xff) as usize] ^ (crc >> 8);
    }
    !crc
}

const fn crc32_tables() -> [[u32; 256]; 8] {
    let mut tables = [[0u32; 256]; 8];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
            bit += 1;
        }
        tables[0][i] = crc;
        i += 1;
    }
    let mut t = 1;
    while t < 8 {
        let mut i = 0;
        while i < 256 {
            let prev = tables[t - 1][i];
            tables[t][i] = tables[0][(prev & 0xff) as usize] ^ (prev >> 8);
            i += 1;
        }
        t += 1;
    }
    tables
}

/// Zigzag-LEB128 varint cursor over one chunk's byte range. Decoders MUST
/// consume exactly the range: both a varint crossing the boundary and
/// trailing undecoded bytes are malformed-file errors (spec §6.7).
pub(crate) struct StreamCursor<'a> {
    data: &'a [u8],
    pub(crate) pos: u64,
    pub(crate) end: u64,
}

impl<'a> StreamCursor<'a> {
    pub(crate) fn new(data: &'a [u8], pos: u64, end: u64) -> Self {
        Self { data, pos, end }
    }

    pub(crate) fn varint(&mut self) -> Result<i32, Error> {
        let mut u: u32 = 0;
        for i in 0..5 {
            if self.pos >= self.end {
                return malformed("truncated varint");
            }
            let b = self.data[self.pos as usize];
            self.pos += 1;
            if i == 4 && b & 0xf0 != 0 {
                return malformed("varint exceeds 32 bits");
            }
            u |= u32::from(b & 0x7f) << (7 * i);
            if b & 0x80 == 0 {
                if i > 0 && b == 0 {
                    return malformed("nonminimal varint");
                }
                return Ok(((u >> 1) ^ (u & 1).wrapping_neg()) as i32);
            }
        }
        malformed("unterminated varint")
    }
}
