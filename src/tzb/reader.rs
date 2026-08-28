//! Container open/validation, directory records, GRID candidates and the
//! in-place point-in-polygon query walk (spec §8), mirroring the Go
//! `embedbin.Reader` over a byte-backed source.

use super::*;
use geometry_rs::{I32Point, Point};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Section {
    pub off: u32,
    pub len: u32,
}

impl Section {
    fn end(self) -> u64 {
        u64::from(self.off) + u64::from(self.len)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GridInfo {
    pub lng_min: i16,
    pub lat_min: i16,
    pub lng_cells: u16,
    pub lat_cells: u16,
    pub cand_count: u32,
    pub cell_count: u32,
    /// Absolute byte offset of the cell-word array.
    pub cells_off: u64,
    /// Absolute byte offset of the candidate array.
    pub candidates_off: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TzRecord {
    pub first: u32,
    pub count: u16,
    pub bbox: BBox,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PolyRecord {
    pub first: u32,
    pub count: u16,
    pub bbox: BBox,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RingRecord {
    pub first: u32,
    pub point_count: u32,
    pub count: u16,
    pub bbox: BBox,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GroupRecord {
    pub first: u32,
    pub point_count: u32,
    pub count: u16,
    pub entry: I32Point,
    pub exit: I32Point,
    pub bbox: BBox,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChunkRecord {
    pub off: u32,
    pub count: u16,
    pub bbox: BBox,
}

/// A validated `.tzb` (E profile) file over borrowed or owned bytes.
pub(crate) struct Reader<'a> {
    data: Cow<'a, [u8]>,
    flags: u32,
    tz_count: u32,
    version: String,
    sections: [Section; SECTION_SLOTS],
    poly_count: u32,
    ring_count: u32,
    op_count: u32,
    group_count: u32,
    chunk_count: u32,
    grid: Option<GridInfo>,
    pub(super) fuzzy: Option<super::fuzzy::FuzzyInfo>,
}

impl<'a> Reader<'a> {
    /// Validates and opens a byte-backed file (spec §8.1 "open" checks; the
    /// per-record checks below run again on every access, so a structurally
    /// broken record can never be consumed).
    pub(crate) fn open(data: Cow<'a, [u8]>) -> Result<Self, Error> {
        let size = data.len() as u64;
        if size < HEADER_SIZE + FOOTER_SIZE || size > u64::from(u32::MAX) {
            return malformed("invalid file size");
        }
        let h = &data[..HEADER_SIZE as usize];
        if &h[0..4] != b"TZFB" || h[4] != FORMAT_MAJOR {
            return malformed("magic or format major");
        }
        // The memory-image (.tzm) profile is a Go-side optimization built on
        // zero-copy ring aliasing; geometry-rs polygons own their storage, so
        // tzf-rs consumes the .tzb profile only (v2 decision, 2026-08-28).
        match h[PROFILE_OFFSET] {
            PROFILE_E => {}
            PROFILE_M => return Err(Error::Profile),
            _ => return malformed("unsupported profile"),
        }
        if u16_le(h, 6) != HEADER_SIZE as u16
            || u32_le(h, 12) != COORD_SCALE
            || u64::from(u32_le(h, 16)) != size
        {
            return malformed("header fields");
        }
        let flags = u32_le(h, 8);
        let tz_count = u32_le(h, 40);
        let chunk_target = u32_le(h, 44);
        if tz_count == 0 || tz_count > u32::from(u16::MAX) {
            return malformed("header counts");
        }
        if chunk_target == 0 || chunk_target > u32::from(u16::MAX) {
            return malformed("header chunk target");
        }
        let version_raw = &h[24..40];
        let version_end = version_raw.iter().position(|&b| b == 0).unwrap_or(16);
        if version_raw[version_end..].iter().any(|&b| b != 0) {
            return malformed("data version padding");
        }
        let version = match std::str::from_utf8(&version_raw[..version_end]) {
            Ok(v) => v.to_string(),
            Err(_) => return malformed("data version UTF-8"),
        };

        let section_count = u32_le(h, 20);
        let table_end = HEADER_SIZE + u64::from(section_count) * SECTION_ENTRY_LEN;
        if table_end > size - FOOTER_SIZE {
            return malformed("section table bounds");
        }
        let footer = u32_le(&data, (size - FOOTER_SIZE) as usize);
        if crc32_ieee(&data[..(size - FOOTER_SIZE) as usize]) != footer {
            return malformed("CRC32");
        }

        let entry_at = |i: u32| -> (u32, Section) {
            let base = (HEADER_SIZE + u64::from(i) * SECTION_ENTRY_LEN) as usize;
            (
                u32_le(&data, base),
                Section {
                    off: u32_le(&data, base + 4),
                    len: u32_le(&data, base + 8),
                },
            )
        };

        let mut sections = [Section::default(); SECTION_SLOTS];
        let mut seen = [false; SECTION_SLOTS];
        for i in 0..section_count {
            let (typ, entry) = entry_at(i);
            if entry.off % 4 != 0 {
                return malformed("unaligned section");
            }
            if u64::from(entry.off) < table_end || entry.end() > size - FOOTER_SIZE {
                return malformed("section bounds");
            }
            if (SECTION_NAMES..SECTION_SLOTS as u32).contains(&typ) {
                let slot = typ as usize;
                if seen[slot] {
                    return malformed("duplicate known section");
                }
                if !section_allowed(typ) {
                    return malformed("section not valid in profile");
                }
                seen[slot] = true;
                sections[slot] = entry;
            }
            for j in 0..i {
                let (_, other) = entry_at(j);
                if ranges_overlap(entry, other) {
                    return malformed("overlapping sections");
                }
            }
        }

        // Mandatory sections of the E profile (pb-free spec §6.1).
        let mandatory: &[u32] = &[
            SECTION_NAMES,
            SECTION_TZDIR,
            SECTION_POLYDIR,
            SECTION_RINGDIR,
            SECTION_RINGOPS,
            SECTION_GROUPDIR,
            SECTION_CHUNKDIR,
            SECTION_POINTS,
        ];
        for &typ in mandatory {
            if !seen[typ as usize] {
                return malformed("missing mandatory section");
            }
        }
        let has_grid = seen[SECTION_GRID as usize];
        if has_grid != (flags & FLAG_GRID != 0) {
            return malformed("GRID flag mismatch");
        }
        if u64::from(sections[SECTION_TZDIR as usize].len) != u64::from(tz_count) * TZ_RECORD_LEN
            || u64::from(sections[SECTION_POLYDIR as usize].len) % POLY_RECORD_LEN != 0
        {
            return malformed("directory section length");
        }

        if u64::from(sections[SECTION_RINGDIR as usize].len) % RING_RECORD_LEN != 0
            || !sections[SECTION_RINGOPS as usize].len.is_multiple_of(4)
            || u64::from(sections[SECTION_GROUPDIR as usize].len) % GROUP_RECORD_LEN != 0
            || u64::from(sections[SECTION_CHUNKDIR as usize].len) % CHUNK_RECORD_LEN != 0
        {
            return malformed("directory section length");
        }
        let mut r = Reader {
            data,
            flags,
            tz_count,
            version,
            sections,
            poly_count: (u64::from(sections[SECTION_POLYDIR as usize].len) / POLY_RECORD_LEN)
                as u32,
            ring_count: (u64::from(sections[SECTION_RINGDIR as usize].len) / RING_RECORD_LEN)
                as u32,
            op_count: sections[SECTION_RINGOPS as usize].len / 4,
            group_count: (u64::from(sections[SECTION_GROUPDIR as usize].len) / GROUP_RECORD_LEN)
                as u32,
            chunk_count: (u64::from(sections[SECTION_CHUNKDIR as usize].len) / CHUNK_RECORD_LEN)
                as u32,
            grid: None,
            fuzzy: None,
        };
        if r.poly_count == 0
            || r.ring_count == 0
            || r.op_count == 0
            || r.group_count == 0
            || r.chunk_count == 0
        {
            return malformed("empty directory");
        }
        r.validate_names()?;
        if has_grid {
            r.validate_grid()?;
        }
        if seen[SECTION_FUZZY as usize] {
            r.validate_fuzzy()?;
        }
        r.validate_chunk_offsets()?;
        Ok(r)
    }

    pub(crate) fn data_version(&self) -> &str {
        &self.version
    }

    pub(crate) fn timezone_count(&self) -> u32 {
        self.tz_count
    }

    pub(crate) fn has_fuzzy(&self) -> bool {
        self.fuzzy.is_some()
    }

    /// A sufficient candidate-buffer capacity for `lookup_into`.
    pub(crate) fn lookup_buffer_size(&self) -> usize {
        if self.grid.is_some() {
            15
        } else {
            self.tz_count as usize
        }
    }

    pub(super) fn raw(&self) -> &[u8] {
        &self.data
    }

    pub(super) fn section(&self, typ: u32) -> Section {
        self.sections[typ as usize]
    }

    /// A bounds-checked byte range.
    fn slice(&self, off: u64, len: u64) -> Result<&[u8], Error> {
        let end = off
            .checked_add(len)
            .ok_or(Error::Malformed("read bounds"))?;
        if end > self.data.len() as u64 {
            return malformed("read bounds");
        }
        Ok(&self.data[off as usize..end as usize])
    }

    fn record(&self, typ: u32, index: u32, count: u32, width: u64) -> Result<&[u8], Error> {
        if index >= count {
            return malformed("directory index");
        }
        let s = self.sections[typ as usize];
        self.slice(u64::from(s.off) + u64::from(index) * width, width)
    }

    // ---- open-time validation ----

    fn validate_names(&self) -> Result<(), Error> {
        let s = self.sections[SECTION_NAMES as usize];
        let prefix = 4 + 4 * (u64::from(self.tz_count) + 1);
        if u64::from(s.len) < prefix {
            return malformed("NAMES length");
        }
        let blob_len = u32_le(self.slice(u64::from(s.off), 4)?, 0);
        if prefix + u64::from(blob_len) != u64::from(s.len) {
            return malformed("NAMES blob length");
        }
        let mut prev = 0u32;
        for i in 0..=self.tz_count {
            let off = u32_le(self.slice(u64::from(s.off) + 4 + u64::from(i) * 4, 4)?, 0);
            if off < prev || off > blob_len || (i == self.tz_count && off != blob_len) {
                return malformed("NAMES offsets");
            }
            if i > 0 && off == prev {
                return malformed("empty timezone name");
            }
            prev = off;
        }
        for i in 0..self.tz_count {
            let name = self.name_bytes(i)?;
            let Ok(text) = std::str::from_utf8(name) else {
                return malformed("invalid name UTF-8");
            };
            if text.contains('\0') {
                return malformed("NUL in name");
            }
        }
        Ok(())
    }

    fn validate_grid(&mut self) -> Result<(), Error> {
        let s = self.sections[SECTION_GRID as usize];
        if s.len < 12 {
            return malformed("GRID length");
        }
        let raw = self.slice(u64::from(s.off), 12)?;
        let lng_min = i16_le(raw, 0);
        let lat_min = i16_le(raw, 2);
        let lng_cells = u16_le(raw, 4);
        let lat_cells = u16_le(raw, 6);
        let cand_count = u32_le(raw, 8);
        if lng_cells == 0
            || lat_cells == 0
            || !(-181..=180).contains(&lng_min)
            || !(-91..=90).contains(&lat_min)
            || i32::from(lng_min) + i32::from(lng_cells) - 1 > 181
            || i32::from(lat_min) + i32::from(lat_cells) - 1 > 91
            || cand_count >= 1 << 28
        {
            return malformed("GRID dimensions");
        }
        let cells = u64::from(lng_cells) * u64::from(lat_cells);
        let expect = 12 + cells * 4 + u64::from(cand_count) * 2;
        if expect != u64::from(s.len) {
            return malformed("GRID section size");
        }
        let g = GridInfo {
            lng_min,
            lat_min,
            lng_cells,
            lat_cells,
            cand_count,
            cell_count: cells as u32,
            cells_off: u64::from(s.off) + 12,
            candidates_off: u64::from(s.off) + 12 + cells * 4,
        };
        for i in 0..g.cell_count {
            let word = u32_le(self.slice(g.cells_off + u64::from(i) * 4, 4)?, 0);
            let (count, off) = (word >> 28, word & 0x0fff_ffff);
            if u64::from(off) + u64::from(count) > u64::from(g.cand_count) {
                return malformed("GRID candidate range");
            }
            for j in 0..count {
                let idx = u16_le(self.slice(g.candidates_off + u64::from(off + j) * 2, 2)?, 0);
                if u32::from(idx) >= self.tz_count {
                    return malformed("GRID candidate index");
                }
            }
        }
        self.grid = Some(g);
        Ok(())
    }

    fn validate_chunk_offsets(&self) -> Result<(), Error> {
        let mut prev = 0u32;
        for i in 0..self.chunk_count {
            let c = self.chunk_at(i)?;
            if c.count == 0
                || c.off >= self.sections[SECTION_POINTS as usize].len
                || (i > 0 && c.off <= prev)
            {
                return malformed("chunk offset or count");
            }
            prev = c.off;
        }
        Ok(())
    }

    // ---- directory records ----

    pub(crate) fn tz_at(&self, index: u32) -> Result<TzRecord, Error> {
        let raw = self.record(SECTION_TZDIR, index, self.tz_count, TZ_RECORD_LEN)?;
        let v = TzRecord {
            first: u32_le(raw, 0),
            count: u16_le(raw, 4),
            bbox: BBox::read(raw, 8),
        };
        if v.count == 0
            || u64::from(v.first) + u64::from(v.count) > u64::from(self.poly_count)
            || !v.bbox.in_domain()
        {
            return malformed("TZDIR record");
        }
        Ok(v)
    }

    pub(crate) fn poly_at(&self, index: u32) -> Result<PolyRecord, Error> {
        let raw = self.record(SECTION_POLYDIR, index, self.poly_count, POLY_RECORD_LEN)?;
        let v = PolyRecord {
            first: u32_le(raw, 0),
            count: u16_le(raw, 4),
            bbox: BBox::read(raw, 8),
        };
        if v.count == 0
            || u64::from(v.first) + u64::from(v.count) > u64::from(self.ring_count)
            || !v.bbox.in_domain()
        {
            return malformed("POLYDIR record");
        }
        Ok(v)
    }

    pub(crate) fn ring_at(&self, index: u32) -> Result<RingRecord, Error> {
        let raw = self.record(SECTION_RINGDIR, index, self.ring_count, RING_RECORD_LEN)?;
        let v = RingRecord {
            first: u32_le(raw, 0),
            point_count: u32_le(raw, 4),
            count: u16_le(raw, 8),
            bbox: BBox::read(raw, 12),
        };
        if v.count == 0
            || v.point_count < 3
            || u64::from(v.first) + u64::from(v.count) > u64::from(self.op_count)
            || !v.bbox.in_domain()
        {
            return malformed("RINGDIR record");
        }
        Ok(v)
    }

    pub(crate) fn op_at(&self, index: u32) -> Result<u32, Error> {
        let raw = self.record(SECTION_RINGOPS, index, self.op_count, 4)?;
        let word = u32_le(raw, 0);
        if word & 0x7fff_ffff >= self.group_count {
            return malformed("RINGOPS group index");
        }
        Ok(word)
    }

    pub(crate) fn group_at(&self, index: u32) -> Result<GroupRecord, Error> {
        let raw = self.record(SECTION_GROUPDIR, index, self.group_count, GROUP_RECORD_LEN)?;
        let v = GroupRecord {
            first: u32_le(raw, 0),
            point_count: u32_le(raw, 4),
            count: u16_le(raw, 8),
            entry: I32Point {
                x: i32_le(raw, 12),
                y: i32_le(raw, 16),
            },
            exit: I32Point {
                x: i32_le(raw, 20),
                y: i32_le(raw, 24),
            },
            bbox: BBox::read(raw, 28),
        };
        if v.count == 0
            || v.point_count < 2
            || u64::from(v.first) + u64::from(v.count) > u64::from(self.chunk_count)
            || !v.bbox.in_domain()
            || !point_in_domain(v.entry)
            || !point_in_domain(v.exit)
        {
            return malformed("GROUPDIR record");
        }
        let mut total = 0u64;
        for i in 0..u32::from(v.count) {
            total += u64::from(self.chunk_at(v.first + i)?.count);
        }
        if total != u64::from(v.point_count) {
            return malformed("group point count");
        }
        Ok(v)
    }

    pub(crate) fn chunk_at(&self, index: u32) -> Result<ChunkRecord, Error> {
        let raw = self.record(SECTION_CHUNKDIR, index, self.chunk_count, CHUNK_RECORD_LEN)?;
        let v = ChunkRecord {
            off: u32_le(raw, 0),
            count: u16_le(raw, 4),
            bbox: BBox::read(raw, 8),
        };
        if v.count == 0
            || v.off >= self.sections[SECTION_POINTS as usize].len
            || !v.bbox.in_domain()
        {
            return malformed("CHUNKDIR record");
        }
        Ok(v)
    }

    // ---- names ----

    fn name_bounds(&self, idx: u32) -> Result<(u64, u64), Error> {
        if idx >= self.tz_count {
            return Err(Error::Index);
        }
        let s = self.sections[SECTION_NAMES as usize];
        let raw = self.slice(u64::from(s.off) + 4 + u64::from(idx) * 4, 8)?;
        let a = u32_le(raw, 0);
        let b = u32_le(raw, 4);
        let base = u64::from(s.off) + 4 + (u64::from(self.tz_count) + 1) * 4;
        Ok((base + u64::from(a), base + u64::from(b)))
    }

    /// A timezone name's raw bytes (validated UTF-8 after open), zero-copy.
    pub(crate) fn name_bytes(&self, idx: u32) -> Result<&[u8], Error> {
        let (start, end) = self.name_bounds(idx)?;
        self.slice(start, end - start)
    }

    pub(crate) fn name(&self, idx: u32) -> Result<&str, Error> {
        std::str::from_utf8(self.name_bytes(idx)?).map_err(|_| Error::Malformed("name UTF-8"))
    }

    // ---- GRID candidates ----

    /// The grid candidate range for a query point: `(count, offset, has_grid)`.
    /// Rejects non-finite and out-of-domain input (spec §8 step 0).
    fn candidates(&self, lng: f64, lat: f64) -> Result<(u32, u32, bool), Error> {
        if !lng.is_finite()
            || !lat.is_finite()
            || !(-180.0..=180.0).contains(&lng)
            || !(-90.0..=90.0).contains(&lat)
        {
            return Ok((0, 0, self.grid.is_some()));
        }
        let Some(g) = &self.grid else {
            return Ok((self.tz_count, 0, false));
        };
        #[allow(clippy::cast_possible_truncation)]
        let cx = lng.floor() as i64 - i64::from(g.lng_min);
        #[allow(clippy::cast_possible_truncation)]
        let cy = lat.floor() as i64 - i64::from(g.lat_min);
        if cx < 0 || cy < 0 || cx >= i64::from(g.lng_cells) || cy >= i64::from(g.lat_cells) {
            return Ok((0, 0, true));
        }
        let cell = cy as u64 * u64::from(g.lng_cells) + cx as u64;
        let word = u32_le(self.slice(g.cells_off + cell * 4, 4)?, 0);
        Ok((word >> 28, word & 0x0fff_ffff, true))
    }

    fn candidate_at(&self, off: u32) -> Result<u32, Error> {
        let g = self.grid.as_ref().ok_or(Error::Malformed("no grid"))?;
        if off >= g.cand_count {
            return malformed("candidate offset");
        }
        let idx = u32::from(u16_le(
            self.slice(g.candidates_off + u64::from(off) * 2, 2)?,
            0,
        ));
        if idx >= self.tz_count {
            return malformed("candidate index");
        }
        Ok(idx)
    }

    // ---- in-place query walk (E profile) ----

    /// Returns the first containing timezone index in source order.
    pub(crate) fn lookup(&self, lng: f64, lat: f64) -> Result<Option<u32>, Error> {
        let (count, off, grid) = self.candidates(lng, lat)?;
        if count == 0 {
            return Ok(None);
        }
        // Single-candidate shortcut (spec §8 step 2).
        if grid
            && count == 1
            && self.flags & FLAG_NO_SHORTCUT == 0
            && lng > -179.0
            && lng < 179.0
            && lat > -89.0
            && lat < 89.0
        {
            return Ok(Some(self.candidate_at(off)?));
        }
        let x = lng * f64::from(COORD_SCALE);
        let y = lat * f64::from(COORD_SCALE);
        for i in 0..count {
            let idx = if grid { self.candidate_at(off + i)? } else { i };
            if self.timezone_contains(idx, x, y)? {
                return Ok(Some(idx));
            }
        }
        Ok(None)
    }

    /// Appends all matching indices to `dst`, sorted lexicographically by
    /// name (raw UTF-8 bytes, spec §8 multi-result rule).
    pub(crate) fn lookup_into(&self, lng: f64, lat: f64, dst: &mut Vec<u32>) -> Result<(), Error> {
        dst.clear();
        let (count, off, grid) = self.candidates(lng, lat)?;
        let x = lng * f64::from(COORD_SCALE);
        let y = lat * f64::from(COORD_SCALE);
        for i in 0..count {
            let idx = if grid { self.candidate_at(off + i)? } else { i };
            if self.timezone_contains(idx, x, y)? {
                dst.push(idx);
            }
        }
        // Names were validated at open; a sort key read cannot fail here.
        dst.sort_by(|&a, &b| {
            self.name_bytes(a)
                .unwrap_or_default()
                .cmp(self.name_bytes(b).unwrap_or_default())
        });
        Ok(())
    }

    fn timezone_contains(&self, index: u32, x: f64, y: f64) -> Result<bool, Error> {
        let t = self.tz_at(index)?;
        if !t.bbox.contains(x, y) {
            return Ok(false);
        }
        for i in 0..u32::from(t.count) {
            let p = self.poly_at(t.first + i)?;
            if !p.bbox.contains(x, y) {
                continue;
            }
            // Exterior rings allow on-edge containment and hole rings do not,
            // matching geometry-rs contains_point_allow_on_edge: a border
            // query belongs to every polygon touching it, and a point on a
            // hole's boundary stays inside the polygon.
            if !self.ring_contains(p.first, x, y, true)? {
                continue;
            }
            let mut excluded = false;
            for h in 1..u32::from(p.count) {
                let hr = self.ring_at(p.first + h)?;
                if !hr.bbox.contains(x, y) {
                    continue;
                }
                if self.ring_contains(p.first + h, x, y, false)? {
                    excluded = true;
                    break;
                }
            }
            if !excluded {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether the ring contains (x, y). A point on any ring segment returns
    /// `allow_on_edge`.
    fn ring_contains(
        &self,
        index: u32,
        x: f64,
        y: f64,
        allow_on_edge: bool,
    ) -> Result<bool, Error> {
        let ring = self.ring_at(index)?;
        if !ring.bbox.contains(x, y) {
            return Ok(false);
        }
        let p = Point { x, y };
        let mut inside = false;
        let mut first_entry = I32Point { x: 0, y: 0 };
        let mut previous_exit = I32Point { x: 0, y: 0 };
        let mut sum = 0u64;
        for i in 0..u32::from(ring.count) {
            let word = self.op_at(ring.first + i)?;
            let group = self.group_at(word & 0x7fff_ffff)?;
            sum += u64::from(group.point_count);
            let (mut entry, mut exit) = (group.entry, group.exit);
            if word >> 31 != 0 {
                std::mem::swap(&mut entry, &mut exit);
            }
            if i == 0 {
                first_entry = entry;
            } else if !same_point(previous_exit, entry) {
                // Junction rule (spec §8): encoder-produced files always
                // connect, but a disconnected junction contributes the real
                // segment expansion would create.
                let (cross, on) = raycast_seg(to_point(previous_exit), to_point(entry), p);
                if on {
                    return Ok(allow_on_edge);
                }
                if cross {
                    inside = !inside;
                }
            }
            previous_exit = exit;
            if group.bbox.ray_relevant(x, y) && self.scan_group(&group, p, &mut inside)? {
                return Ok(allow_on_edge);
            }
        }
        if sum < u64::from(ring.count) || sum - u64::from(ring.count) != u64::from(ring.point_count)
        {
            return malformed("ring point count");
        }
        if !same_point(previous_exit, first_entry) {
            let (cross, on) = raycast_seg(to_point(previous_exit), to_point(first_entry), p);
            if on {
                return Ok(allow_on_edge);
            }
            if cross {
                inside = !inside;
            }
        }
        Ok(inside)
    }

    /// Scans one group's chunks; returns true when p lies on a segment.
    fn scan_group(&self, group: &GroupRecord, p: Point, inside: &mut bool) -> Result<bool, Error> {
        for i in 0..u32::from(group.count) {
            let chunk_index = group.first + i;
            let chunk = self.chunk_at(chunk_index)?;
            if !chunk.bbox.ray_relevant(p.x, p.y) {
                continue;
            }
            let (last, on) = self.scan_chunk(chunk_index, chunk, p, inside)?;
            if on {
                return Ok(true);
            }
            if i + 1 < u32::from(group.count) {
                // Joint segment to the next chunk's first point.
                let next = self.chunk_at(chunk_index + 1)?;
                let first = self.first_chunk_point(chunk_index + 1, next)?;
                let (cross, on) = raycast_seg(to_point(last), to_point(first), p);
                if on {
                    return Ok(true);
                }
                if cross {
                    *inside = !*inside;
                }
            }
        }
        Ok(false)
    }

    /// Evaluates one chunk's internal segments; returns the chunk's last
    /// point and whether p lay on any segment.
    fn scan_chunk(
        &self,
        index: u32,
        chunk: ChunkRecord,
        p: Point,
        inside: &mut bool,
    ) -> Result<(I32Point, bool), Error> {
        let (start, end) = self.chunk_range(index, chunk)?;
        let mut cursor = StreamCursor::new(&self.data, start, end);
        let mut prev = I32Point {
            x: cursor.varint()?,
            y: cursor.varint()?,
        };
        if !point_in_domain(prev) {
            return malformed("chunk coordinate domain");
        }
        let mut on_segment = false;
        for _ in 1..chunk.count {
            let dx = cursor.varint()?;
            let dy = cursor.varint()?;
            let next = I32Point {
                x: add_delta(prev.x, dx)?,
                y: add_delta(prev.y, dy)?,
            };
            if !point_in_domain(next) {
                return malformed("chunk coordinate domain");
            }
            if !on_segment {
                let (cross, on) = raycast_seg(to_point(prev), to_point(next), p);
                if on {
                    on_segment = true;
                } else if cross {
                    *inside = !*inside;
                }
            }
            prev = next;
        }
        if cursor.pos != end {
            return malformed("trailing chunk bytes");
        }
        Ok((prev, on_segment))
    }

    pub(crate) fn first_chunk_point(
        &self,
        index: u32,
        chunk: ChunkRecord,
    ) -> Result<I32Point, Error> {
        let (start, end) = self.chunk_range(index, chunk)?;
        let mut cursor = StreamCursor::new(&self.data, start, end);
        let p = I32Point {
            x: cursor.varint()?,
            y: cursor.varint()?,
        };
        if !point_in_domain(p) {
            return malformed("chunk coordinate domain");
        }
        Ok(p)
    }

    /// The absolute byte range of one chunk's stream: `[point_off_k,
    /// point_off_{k+1})`, the last chunk ending at the POINTS section end.
    fn chunk_range(&self, index: u32, chunk: ChunkRecord) -> Result<(u64, u64), Error> {
        let points = self.sections[SECTION_POINTS as usize];
        let start = u64::from(points.off) + u64::from(chunk.off);
        let end = if index + 1 < self.chunk_count {
            u64::from(points.off) + u64::from(self.chunk_at(index + 1)?.off)
        } else {
            points.end()
        };
        if start >= end {
            return malformed("chunk byte range");
        }
        Ok((start, end))
    }

    /// Decodes one chunk's full point run.
    pub(crate) fn decode_chunk_points(
        &self,
        index: u32,
        chunk: ChunkRecord,
        out: &mut Vec<I32Point>,
    ) -> Result<(), Error> {
        let (start, end) = self.chunk_range(index, chunk)?;
        let mut cursor = StreamCursor::new(&self.data, start, end);
        let mut prev = I32Point {
            x: cursor.varint()?,
            y: cursor.varint()?,
        };
        if !point_in_domain(prev) {
            return malformed("point domain");
        }
        out.push(prev);
        for _ in 1..chunk.count {
            let dx = cursor.varint()?;
            let dy = cursor.varint()?;
            prev = I32Point {
                x: add_delta(prev.x, dx)?,
                y: add_delta(prev.y, dy)?,
            };
            if !point_in_domain(prev) {
                return malformed("point domain");
            }
            out.push(prev);
        }
        if cursor.pos != end {
            return malformed("chunk termination");
        }
        Ok(())
    }

    /// Copies the dense GRID arrays out of the file: `(info, cell words,
    /// candidates)`. `None` when the file has no GRID section.
    pub(crate) fn grid_arrays(&self) -> Option<(GridInfo, Vec<u32>, Vec<u16>)> {
        let g = *self.grid.as_ref()?;
        let mut words = Vec::with_capacity(g.cell_count as usize);
        for i in 0..g.cell_count {
            words.push(u32_le(
                &self.data,
                (g.cells_off + u64::from(i) * 4) as usize,
            ));
        }
        let mut cands = Vec::with_capacity(g.cand_count as usize);
        for i in 0..g.cand_count {
            cands.push(u16_le(
                &self.data,
                (g.candidates_off + u64::from(i) * 2) as usize,
            ));
        }
        Some((g, words, cands))
    }

    pub(super) fn group_count(&self) -> u32 {
        self.group_count
    }

    pub(super) fn ring_count(&self) -> u32 {
        self.ring_count
    }
}

/// Whether a known section type may appear in an E-profile file (pb-free
/// spec §6.1): the M-profile section types are structurally invalid here.
fn section_allowed(typ: u32) -> bool {
    !matches!(
        typ,
        SECTION_FLAT_POINTS | SECTION_FLAT_RING_DIR | SECTION_YSTRIPES
    )
}

fn ranges_overlap(a: Section, b: Section) -> bool {
    u64::from(a.off) < b.end() && u64::from(b.off) < a.end()
}

fn to_point(p: I32Point) -> Point {
    Point {
        x: f64::from(p.x),
        y: f64::from(p.y),
    }
}
