//! Slippy-map tile arithmetic, ported from the Go `internal/geom` package so
//! FUZZY lookups compute bit-identical tile keys to the preindex encoder.

use std::f64::consts::PI;

/// Returns the slippy-map tile column (x) and row (y) for (lng, lat) at the
/// given zoom level, using the Web Mercator projection (OSM convention).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn tile_xy(lng: f64, lat: f64, zoom: u32) -> (u32, u32) {
    let n = f64::from(1u32 << zoom);
    let x = ((lng / 360.0 + 0.5) * n) as u32;
    let y = if lat > 85.0511 {
        0
    } else if lat < -85.0511 {
        (n as u32) - 1
    } else {
        let siny = (lat * PI / 180.0).sin();
        ((0.5 - ((1.0 + siny) / (1.0 - siny)).ln() / (4.0 * PI)) * n) as u32
    };
    (x, y)
}

/// Packs (x, y, z) into a single key.
/// Layout: bits 56-63 = zoom (0-255), bits 28-55 = x, bits 0-27 = y.
/// This covers all OSM zoom levels (0-28) without collision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TileId(pub u64);

impl TileId {
    pub(crate) fn new(lng: f64, lat: f64, zoom: u32) -> Self {
        let (x, y) = tile_xy(lng, lat, zoom);
        Self::from_xyz(x, y, zoom as u8)
    }

    pub(crate) fn from_xyz(x: u32, y: u32, z: u8) -> Self {
        Self(u64::from(z) << 56 | u64::from(x) << 28 | u64::from(y))
    }

    pub(crate) fn xyz(self) -> (u32, u32, u8) {
        (
            (self.0 >> 28) as u32 & 0x0FFF_FFFF,
            self.0 as u32 & 0x0FFF_FFFF,
            (self.0 >> 56) as u8,
        )
    }

    /// The tile at a coarser zoom: right-shift the high-zoom tile coordinates
    /// instead of repeating the transcendental math.
    pub(crate) fn shift(self, shift: u8) -> Self {
        let (x, y, z) = self.xyz();
        if shift > z {
            return Self(0);
        }
        Self::from_xyz(x >> shift, y >> shift, z - shift)
    }
}
