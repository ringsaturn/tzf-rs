//! FUZZY section (type 10): the preindex tile set stored as one sorted key
//! array (pb-free spec §4). Validated at open; queried in place by the
//! embedded finder and materialized into hash maps by the composed finders.

use super::reader::Reader;
use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct FuzzyInfo {
    pub idx_zoom: u8,
    pub agg_zoom: u8,
    pub tile_count: u32,
    pub multi_group_count: u32,
    pub multi_value_count: u32,
    /// Absolute byte offsets.
    pub keys_off: u64,
    pub values_off: u64,
    pub multi_dir_off: u64,
    pub multi_values_off: u64,
    pub max_group_len: u32,
}

impl Reader<'_> {
    /// Checks the FUZZY section's structure: exact length for the stored
    /// counts, ordered keys, and resolvable values. Semantic parity with the
    /// source preindex is the build pipeline's job (spec §8.1 trust model).
    pub(super) fn validate_fuzzy(&mut self) -> Result<(), Error> {
        let s = self.section(SECTION_FUZZY);
        if !s.off.is_multiple_of(8) {
            return malformed("FUZZY section alignment");
        }
        if u64::from(s.len) < FUZZY_HEADER_LEN {
            return malformed("FUZZY length");
        }
        let raw = &self.raw()[s.off as usize..s.off as usize + FUZZY_HEADER_LEN as usize];
        let mut f = FuzzyInfo {
            idx_zoom: raw[0],
            agg_zoom: raw[1],
            tile_count: u32_le(raw, 4),
            multi_group_count: u32_le(raw, 8),
            multi_value_count: u32_le(raw, 12),
            keys_off: 0,
            values_off: 0,
            multi_dir_off: 0,
            multi_values_off: 0,
            max_group_len: 1,
        };
        if raw[2] != 0
            || raw[3] != 0
            || f.agg_zoom > f.idx_zoom
            || f.idx_zoom > 28
            || f.tile_count == 0
        {
            return malformed("FUZZY header");
        }
        let size = FUZZY_HEADER_LEN
            + 8 * u64::from(f.tile_count)
            + 2 * u64::from(f.tile_count)
            + 4 * u64::from(f.multi_group_count)
            + 2 * u64::from(f.multi_value_count);
        if align4(size) != u64::from(s.len) {
            return malformed("FUZZY section size");
        }
        f.keys_off = u64::from(s.off) + FUZZY_HEADER_LEN;
        f.values_off = f.keys_off + 8 * u64::from(f.tile_count);
        f.multi_dir_off = f.values_off + 2 * u64::from(f.tile_count);
        f.multi_values_off = f.multi_dir_off + 4 * u64::from(f.multi_group_count);
        let data = self.raw();
        for pad in size..u64::from(s.len) {
            if data[(u64::from(s.off) + pad) as usize] != 0 {
                return malformed("FUZZY padding");
            }
        }

        let mut prev = 0u64;
        for i in 0..f.tile_count {
            let key = u64_le(data, (f.keys_off + 8 * u64::from(i)) as usize);
            if i > 0 && key <= prev {
                return malformed("FUZZY keys not strictly ascending");
            }
            prev = key;
            let z = (key >> 56) as u8;
            if z < f.agg_zoom || z > f.idx_zoom {
                return malformed("FUZZY key zoom");
            }
            let value = u16_le(data, (f.values_off + 2 * u64::from(i)) as usize);
            if value & FUZZY_MULTI == 0 {
                if u32::from(value) >= self.timezone_count() {
                    return malformed("FUZZY value index");
                }
            } else if u32::from(value & !FUZZY_MULTI) >= f.multi_group_count {
                return malformed("FUZZY multi group index");
            }
        }
        for g in 0..f.multi_group_count {
            let off = (f.multi_dir_off + 4 * u64::from(g)) as usize;
            let first = u16_le(data, off);
            let count = u16_le(data, off + 2);
            if count == 0 || u32::from(first) + u32::from(count) > f.multi_value_count {
                return malformed("FUZZY multi group range");
            }
            f.max_group_len = f.max_group_len.max(u32::from(count));
        }
        for i in 0..f.multi_value_count {
            let v = u16_le(data, (f.multi_values_off + 2 * u64::from(i)) as usize);
            if u32::from(v) >= self.timezone_count() {
                return malformed("FUZZY multi value index");
            }
        }
        self.fuzzy = Some(f);
        Ok(())
    }

    fn fuzzy_key_at(&self, f: &FuzzyInfo, i: u32) -> u64 {
        u64_le(self.raw(), (f.keys_off + 8 * u64::from(i)) as usize)
    }

    fn fuzzy_group_at(&self, f: &FuzzyInfo, g: u32) -> (u16, u16) {
        let off = (f.multi_dir_off + 4 * u64::from(g)) as usize;
        (u16_le(self.raw(), off), u16_le(self.raw(), off + 2))
    }

    /// Binary-searches the sorted key array. Because zoom occupies the key's
    /// high bits, a per-zoom probe is a single search.
    fn fuzzy_search(&self, f: &FuzzyInfo, target: u64) -> Option<u32> {
        let (mut lo, mut hi) = (0u32, f.tile_count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.fuzzy_key_at(f, mid) < target {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo < f.tile_count && self.fuzzy_key_at(f, lo) == target {
            Some(lo)
        } else {
            None
        }
    }

    /// Walks zoom levels coarsest-first and returns the first hit's value
    /// word: the tile-map lookup loop with the hash maps replaced by one
    /// sorted array.
    fn fuzzy_probe(&self, lng: f64, lat: f64) -> Result<Option<u16>, Error> {
        let Some(f) = &self.fuzzy else {
            return Err(Error::NoFuzzy);
        };
        if !lng.is_finite()
            || !lat.is_finite()
            || !(-180.0..=180.0).contains(&lng)
            || !(-90.0..=90.0).contains(&lat)
        {
            return Ok(None);
        }
        let tile = TileId::new(lng, lat, u32::from(f.idx_zoom));
        for z in f.agg_zoom..=f.idx_zoom {
            let key = tile.shift(f.idx_zoom - z).0;
            if let Some(pos) = self.fuzzy_search(f, key) {
                let value = u16_le(self.raw(), (f.values_off + 2 * u64::from(pos)) as usize);
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    /// The FUZZY tile match for (lng, lat), if any. Multi-name tiles resolve
    /// to the group's first entry (first-listed wins).
    pub(crate) fn fuzzy_lookup(&self, lng: f64, lat: f64) -> Result<Option<u32>, Error> {
        let Some(value) = self.fuzzy_probe(lng, lat)? else {
            return Ok(None);
        };
        if value & FUZZY_MULTI == 0 {
            return Ok(Some(u32::from(value)));
        }
        let f = self.fuzzy.as_ref().ok_or(Error::NoFuzzy)?;
        let (first, _) = self.fuzzy_group_at(f, u32::from(value & !FUZZY_MULTI));
        let v = u16_le(
            self.raw(),
            (f.multi_values_off + 2 * u64::from(first)) as usize,
        );
        Ok(Some(u32::from(v)))
    }

    /// Materializes the FUZZY section into `(key, indices)` pairs for the
    /// hash-map fast path the composed finders use: one pass over the sorted
    /// key array, multi groups keeping their stored order.
    pub(crate) fn fuzzy_entries(&self) -> Result<Vec<(u64, Vec<u16>)>, Error> {
        let Some(f) = &self.fuzzy else {
            return Err(Error::NoFuzzy);
        };
        let data = self.raw();
        let mut out = Vec::with_capacity(f.tile_count as usize);
        for i in 0..f.tile_count {
            let key = self.fuzzy_key_at(f, i);
            let value = u16_le(data, (f.values_off + 2 * u64::from(i)) as usize);
            if value & FUZZY_MULTI == 0 {
                out.push((key, vec![value]));
                continue;
            }
            let (first, count) = self.fuzzy_group_at(f, u32::from(value & !FUZZY_MULTI));
            let group = (0..u32::from(count))
                .map(|j| {
                    u16_le(
                        data,
                        (f.multi_values_off + 2 * (u64::from(first) + u64::from(j))) as usize,
                    )
                })
                .collect();
            out.push((key, group));
        }
        Ok(out)
    }

    /// The FUZZY section's `(idx_zoom, agg_zoom)`.
    pub(crate) fn fuzzy_zooms(&self) -> Option<(u8, u8)> {
        self.fuzzy.as_ref().map(|f| (f.idx_zoom, f.agg_zoom))
    }
}
