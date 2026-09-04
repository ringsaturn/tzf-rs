//! Expansion load path (pb-free spec §5.1): decode an E-profile file's
//! geometry into open per-ring point slices, the exact inputs of the
//! materialized int32 polygon finder.

use super::reader::Reader;
use super::*;
use geometry_rs::I32Point;

/// One polygon's rings in open form (no closing vertex).
pub(crate) struct ExpandedPolygon {
    pub exterior: Vec<I32Point>,
    pub holes: Vec<Vec<I32Point>>,
}

/// The decode-once outputs of an E-profile file.
pub(crate) struct Expanded {
    pub version: String,
    pub names: Vec<String>,
    /// Indexed by timezone, parallel to `names`.
    pub polygons: Vec<Vec<ExpandedPolygon>>,
}

impl Reader<'_> {
    /// Decodes the file's geometry in one sequential pass. Stored junction
    /// duplicates are removed: each op after the first drops its first
    /// streamed point, and the ring's final closing point is dropped, so ring
    /// length equals `RINGDIR.point_count`. The removed vertices are
    /// zero-length PIP no-ops, so queries over the result match the in-place
    /// reader; exported vertex lists simply omit duplicates.
    pub(crate) fn expand(&self) -> Result<Expanded, Error> {
        let group_count = self.group_count();
        let mut groups: Vec<Vec<I32Point>> = Vec::with_capacity(group_count as usize);
        for i in 0..group_count {
            groups.push(self.decode_group_at(i)?);
        }

        let ring_count = self.ring_count();
        let mut rings: Vec<Vec<I32Point>> = Vec::with_capacity(ring_count as usize);
        for i in 0..ring_count {
            rings.push(self.expand_ring(i, |g| Ok(&groups[g as usize]))?);
        }

        let mut names = Vec::with_capacity(self.timezone_count() as usize);
        let mut polygons = Vec::with_capacity(self.timezone_count() as usize);
        // Rings are consumed exactly once (poly ranges partition RINGDIR), so
        // moving them out avoids a second copy of the whole geometry.
        let mut rings: Vec<Option<Vec<I32Point>>> = rings.into_iter().map(Some).collect();
        let mut take_ring = |idx: u32| -> Result<Vec<I32Point>, Error> {
            rings
                .get_mut(idx as usize)
                .and_then(Option::take)
                .ok_or(Error::Malformed("ring shared between polygons"))
        };
        for i in 0..self.timezone_count() {
            names.push(self.name(i)?.to_string());
            let t = self.tz_at(i)?;
            let mut polys = Vec::with_capacity(usize::from(t.count));
            for j in 0..u32::from(t.count) {
                let p = self.poly_at(t.first + j)?;
                let exterior = take_ring(p.first)?;
                let mut holes = Vec::with_capacity(usize::from(p.count) - 1);
                for h in 1..u32::from(p.count) {
                    holes.push(take_ring(p.first + h)?);
                }
                polys.push(ExpandedPolygon { exterior, holes });
            }
            polygons.push(polys);
        }
        Ok(Expanded {
            version: self.data_version().to_string(),
            names,
            polygons,
        })
    }

    /// Decodes one timezone's polygons, with the same per-ring result
    /// `expand` produces for that timezone. Only the shared-edge groups its
    /// rings reference are decoded — each at most once — so exporting a
    /// single timezone costs a fraction of a full expansion.
    #[cfg(feature = "export-geojson")]
    pub(crate) fn expand_timezone(&self, index: u32) -> Result<Vec<ExpandedPolygon>, Error> {
        if index >= self.timezone_count() {
            return Err(Error::Index);
        }
        let mut decoded: std::collections::HashMap<u32, Vec<I32Point>> =
            std::collections::HashMap::new();
        let t = self.tz_at(index)?;
        let mut polys = Vec::with_capacity(usize::from(t.count));
        for j in 0..u32::from(t.count) {
            let p = self.poly_at(t.first + j)?;
            let mut ring_of = |idx: u32| -> Result<Vec<I32Point>, Error> {
                // Two closures cannot share `decoded` mutably, so resolve the
                // ring's groups inline: fill the cache first, then expand.
                let ring = self.ring_at(idx)?;
                for k in 0..u32::from(ring.count) {
                    let word = self.op_at(ring.first + k)?;
                    let g = word & 0x7fff_ffff;
                    if let std::collections::hash_map::Entry::Vacant(e) = decoded.entry(g) {
                        e.insert(self.decode_group_at(g)?);
                    }
                }
                self.expand_ring(idx, |g| {
                    decoded
                        .get(&g)
                        .map(Vec::as_slice)
                        .ok_or(Error::Malformed("group cache"))
                })
            };
            let exterior = ring_of(p.first)?;
            let mut holes = Vec::with_capacity(usize::from(p.count) - 1);
            for h in 1..u32::from(p.count) {
                holes.push(ring_of(p.first + h)?);
            }
            polys.push(ExpandedPolygon { exterior, holes });
        }
        Ok(polys)
    }

    /// Decodes one GROUPDIR entry's chunks into its point run and checks the
    /// run against the record's stored endpoints and count.
    fn decode_group_at(&self, index: u32) -> Result<Vec<I32Point>, Error> {
        let g = self.group_at(index)?;
        // Cap the preallocation: point_count is file-controlled, so a forged
        // header must not demand memory before decode proves the data exists.
        let mut points = Vec::with_capacity(g.point_count.min(1 << 16) as usize);
        for j in 0..u32::from(g.count) {
            let idx = g.first + j;
            let chunk = self.chunk_at(idx)?;
            self.decode_chunk_points(idx, chunk, &mut points)?;
        }
        if points.len() as u64 != u64::from(g.point_count)
            || !same_point(points[0], g.entry)
            || !same_point(points[points.len() - 1], g.exit)
        {
            return malformed("group endpoints or count");
        }
        Ok(points)
    }

    /// Assembles one ring from its ops, skipping the duplicated junction
    /// vertex at each op boundary and the stored closing vertex.
    fn expand_ring<'g>(
        &self,
        index: u32,
        mut group: impl FnMut(u32) -> Result<&'g [I32Point], Error>,
    ) -> Result<Vec<I32Point>, Error> {
        let ring = self.ring_at(index)?;
        let mut pts: Vec<I32Point> =
            Vec::with_capacity((u64::from(ring.point_count) + 1).min(1 << 16) as usize);
        for k in 0..u32::from(ring.count) {
            let word = self.op_at(ring.first + k)?;
            let g = group(word & 0x7fff_ffff)?;
            let reversed = word >> 31 != 0;
            let skip = k > 0;
            if skip {
                let entry = if reversed { g[g.len() - 1] } else { g[0] };
                if !same_point(entry, pts[pts.len() - 1]) {
                    return malformed("junction mismatch");
                }
            }
            if reversed {
                // Ring order is the stored order reversed; the junction
                // duplicate to skip is the *last* stored point.
                pts.extend(g.iter().rev().skip(usize::from(skip)).copied());
            } else if skip {
                pts.extend_from_slice(&g[1..]);
            } else {
                pts.extend_from_slice(g);
            }
        }
        if pts.len() as u64 != u64::from(ring.point_count) + 1 {
            return malformed("ring point count");
        }
        if !same_point(pts[pts.len() - 1], pts[0]) {
            return malformed("closing junction mismatch");
        }
        pts.pop();
        Ok(pts)
    }
}
