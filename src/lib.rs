//! Fast timezone finder for Rust: convert (longitude, latitude) coordinates
//! to timezone names, offline.
//!
//! Version 2 is protobuf-free: the data source is the TZF embedded binary
//! format (`.tzb`) shipped by [tzf-dist], and two finder mechanisms consume
//! it:
//!
//! - [`DefaultFinder`] — the recommended general-purpose finder. Expands the
//!   file's geometry into materialized polygons at load time and answers most
//!   queries from the FUZZY preindex tiles, falling back to exact
//!   point-in-polygon for boundary cases.
//! - [`EmbeddedFinder`] — the low-memory finder. Queries the `.tzb` bytes in
//!   place (no expansion, roughly the file size plus ~1 KB of state), with
//!   the same FUZZY fast path. Queries are slower than [`DefaultFinder`];
//!   results are identical.
//!
//! ```rust
//! use tzf_rs::DefaultFinder;
//!
//! let finder = DefaultFinder::new();
//! assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
//! ```
//!
//! Creating a finder is expensive — build one and share it (e.g. via
//! `std::sync::LazyLock`).
//!
//! [tzf-dist]: https://github.com/ringsaturn/tzf-dist
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::borrow::Cow;
use std::f64::consts::PI;

#[cfg(all(feature = "bundled", feature = "full"))]
compile_error!(
    "feature `bundled` is mutually exclusive with the git-only data feature \
     `full`; add `default-features = false` when enabling it"
);

mod finder;
mod tzb;

#[cfg(feature = "export-geojson")]
mod geojson;
#[cfg(feature = "export-geojson")]
pub use geojson::{
    BoundaryFile, FeatureItem, GeometryDefine, MultiPolygonCoordinates, PolygonCoordinates,
    PropertiesDefine,
};

pub use tzb::Error;

use finder::{DenseGrid, FuzzyIndex, PolyFinder, assemble_items};
use tzb::Reader;

#[cfg(feature = "bundled")]
use tzf_dist::load_lite_tzb;
#[cfg(all(not(feature = "bundled"), feature = "full"))]
use tzf_dist_git::load_lite_tzb;

/// The recommended finder: FUZZY preindex fast path over materialized
/// polygon geometry, loaded from a `.tzb` file.
///
/// `get_tz_name` answers from the preindex tile when one covers the point
/// (the vast majority of queries) and falls back to exact point-in-polygon
/// otherwise; `get_tz_names` always uses the polygon scan, as the
/// polygon-exact escape hatch. Files without a FUZZY section get the plain
/// polygon finder.
pub struct DefaultFinder {
    fuzzy: Option<FuzzyIndex>,
    finder: PolyFinder,
}

impl DefaultFinder {
    /// Creates the finder from the bundled tzf-dist lite `.tzb` data.
    ///
    /// # Panics
    ///
    /// Panics when the embedded dataset is malformed — the release pipeline
    /// validates it, so this only fires on a broken build.
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    #[cfg(any(feature = "bundled", feature = "full"))]
    #[must_use]
    pub fn new() -> Self {
        Self::from_tzb(load_lite_tzb()).expect("tzf-dist lite.tzb is validated at release")
    }

    /// Creates the finder from the full-precision `.tzb` dataset (~14 MB,
    /// no topology simplification). Higher fidelity, larger memory footprint.
    ///
    /// Requires the `full` feature, which is git-only:
    /// ```toml
    /// tzf-rs = { git = "https://github.com/ringsaturn/tzf-rs", features = ["full"], default-features = false }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when the embedded dataset is malformed (release-validated).
    #[cfg(feature = "full")]
    #[cfg_attr(docsrs, doc(cfg(feature = "full")))]
    #[must_use]
    pub fn new_full() -> Self {
        Self::from_tzb(tzf_dist_git::load_full_tzb())
            .expect("tzf-dist full.tzb is validated at release")
    }

    /// Builds a finder from TZF embedded binary (`.tzb`) bytes by expanding
    /// the geometry into the materialized polygon engine at load time. `data`
    /// is only read during loading. When the file carries a FUZZY section it
    /// becomes the `get_tz_name` fast path.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the bytes are not a structurally valid
    /// E-profile (`.tzb`) file.
    pub fn from_tzb(data: &[u8]) -> Result<Self, Error> {
        let reader = Reader::open(Cow::Borrowed(data))?;
        let fuzzy = FuzzyIndex::from_reader(&reader)?;
        let grid = DenseGrid::from_reader(&reader);
        let expanded = reader.expand()?;
        Ok(Self {
            fuzzy,
            finder: PolyFinder {
                items: assemble_items(expanded.names, expanded.polygons),
                grid,
                version: expanded.version,
            },
        })
    }

    /// Returns the first matching timezone name, or `""` when no timezone
    /// covers the point.
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        if let Some(fuzzy) = &self.fuzzy
            && let Some(idx) = fuzzy.get(lng, lat)
        {
            return &self.finder.items[usize::from(idx)].name;
        }
        self.finder.get_tz_name(lng, lat)
    }

    /// Returns all matching timezone names (overlapping areas produce more
    /// than one), sorted lexicographically. Always polygon-exact.
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// println!("{:?}", finder.get_tz_names(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        self.finder.get_tz_names(lng, lat)
    }

    /// Returns all timezone names in the dataset.
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// println!("{:?}", finder.timezonenames());
    /// ```
    #[must_use]
    pub fn timezonenames(&self) -> Vec<&str> {
        self.finder.timezonenames()
    }

    /// Returns the dataset release this finder was built from (e.g. `2026c`).
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// println!("{:?}", finder.data_version());
    /// ```
    #[must_use]
    pub fn data_version(&self) -> &str {
        &self.finder.version
    }

    /// Converts all timezone boundaries to a GeoJSON FeatureCollection.
    #[cfg(feature = "export-geojson")]
    #[cfg_attr(docsrs, doc(cfg(feature = "export-geojson")))]
    #[must_use]
    pub fn to_geojson(&self) -> BoundaryFile {
        geojson::collection(
            self.finder
                .items
                .iter()
                .map(geojson::feature_from_item)
                .collect(),
        )
    }

    /// Converts one timezone's boundaries to a GeoJSON FeatureCollection.
    /// Returns `None` when the dataset does not contain the name.
    #[cfg(feature = "export-geojson")]
    #[cfg_attr(docsrs, doc(cfg(feature = "export-geojson")))]
    #[must_use]
    pub fn get_tz_geojson(&self, timezone_name: &str) -> Option<BoundaryFile> {
        let features: Vec<_> = self
            .finder
            .items
            .iter()
            .filter(|item| item.name == timezone_name)
            .map(geojson::feature_from_item)
            .collect();
        if features.is_empty() {
            None
        } else {
            Some(geojson::collection(features))
        }
    }

    /// Converts one timezone's FUZZY preindex tiles to a GeoJSON
    /// FeatureCollection: one Feature whose MultiPolygon holds each tile's
    /// bounding rectangle — the area where `get_tz_name` answers from the
    /// preindex fast path instead of exact point-in-polygon. Tiles are
    /// ordered coarsest zoom first.
    ///
    /// Returns `None` when the file carries no FUZZY section, the dataset
    /// does not contain the name, or no preindex tile names it.
    #[cfg(feature = "export-geojson")]
    #[cfg_attr(docsrs, doc(cfg(feature = "export-geojson")))]
    #[must_use]
    pub fn get_tz_preindex_geojson(&self, timezone_name: &str) -> Option<BoundaryFile> {
        let fuzzy = self.fuzzy.as_ref()?;
        // The same name may map to more than one directory item; a preindex
        // tile may name any of them.
        let indices: Vec<u16> = self
            .finder
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.name == timezone_name)
            .filter_map(|(i, _)| u16::try_from(i).ok())
            .collect();
        if indices.is_empty() {
            return None;
        }
        let keys = fuzzy.tile_keys_for(&indices);
        if keys.is_empty() {
            return None;
        }
        Some(geojson::collection(vec![geojson::feature_from_tile_keys(
            timezone_name.to_string(),
            &keys,
        )]))
    }

    /// Converts the whole FUZZY preindex to a GeoJSON FeatureCollection: one
    /// Feature per timezone that owns at least one tile, in dataset order; a
    /// boundary tile appears in every timezone it names. Returns `None` when
    /// the file carries no FUZZY section.
    #[cfg(feature = "export-geojson")]
    #[cfg_attr(docsrs, doc(cfg(feature = "export-geojson")))]
    #[must_use]
    pub fn to_preindex_geojson(&self) -> Option<BoundaryFile> {
        let fuzzy = self.fuzzy.as_ref()?;
        let mut grouped = fuzzy.tile_keys_grouped();
        let features = self
            .finder
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                let keys = grouped.remove(&u16::try_from(i).ok()?)?;
                Some(geojson::feature_from_tile_keys(item.name.clone(), &keys))
            })
            .collect();
        Some(geojson::collection(features))
    }
}

#[cfg(any(feature = "bundled", feature = "full"))]
impl Default for DefaultFinder {
    fn default() -> Self {
        Self::new()
    }
}

/// The low-memory finder: queries TZF embedded binary (`.tzb`) bytes in
/// place, without expanding the geometry. Total footprint is roughly the file
/// itself (the bundled lite data is ~4 MB) plus ~1 KB of state.
///
/// `get_tz_name` consults the file's FUZZY preindex first and falls back to
/// the compressed-geometry scan; results match [`DefaultFinder`] over the
/// same file, only slower (microseconds instead of hundreds of nanoseconds
/// on boundary queries).
pub struct EmbeddedFinder {
    reader: Reader<'static>,
    names: Vec<String>,
}

impl EmbeddedFinder {
    /// Creates the finder over the bundled tzf-dist lite `.tzb` data,
    /// borrowed in place from the executable's read-only data segment.
    ///
    /// # Panics
    ///
    /// Panics when the embedded dataset is malformed (release-validated).
    #[cfg(any(feature = "bundled", feature = "full"))]
    #[must_use]
    pub fn new() -> Self {
        Self::from_tzb(load_lite_tzb()).expect("tzf-dist lite.tzb is validated at release")
    }

    /// Builds a finder that queries `data` in place. Accepts borrowed
    /// `&'static [u8]` (e.g. `include_bytes!`) without copying, or an owned
    /// `Vec<u8>` (e.g. a file read at startup).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the bytes are not a structurally valid
    /// E-profile (`.tzb`) file. Memory images (`.tzm`) are rejected with
    /// [`Error::Profile`] — they exist for the Go runtime and offer no
    /// benefit here.
    pub fn from_tzb(data: impl Into<Cow<'static, [u8]>>) -> Result<Self, Error> {
        let reader = Reader::open(data.into())?;
        let names = (0..reader.timezone_count())
            .map(|i| reader.name(i).map(str::to_string))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { reader, names })
    }

    /// Returns the first matching timezone name, or `""` when no timezone
    /// covers the point.
    ///
    /// ```rust
    /// use tzf_rs::EmbeddedFinder;
    /// let finder = EmbeddedFinder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        if self.reader.has_fuzzy()
            && let Ok(Some(idx)) = self.reader.fuzzy_lookup(lng, lat)
        {
            return &self.names[idx as usize];
        }
        match self.reader.lookup(lng, lat) {
            Ok(Some(idx)) => &self.names[idx as usize],
            _ => "",
        }
    }

    /// Returns all matching timezone names, sorted lexicographically. Always
    /// polygon-exact.
    #[must_use]
    pub fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        let mut indices = Vec::with_capacity(self.reader.lookup_buffer_size());
        if self.reader.lookup_into(lng, lat, &mut indices).is_err() {
            return Vec::new();
        }
        indices
            .into_iter()
            .map(|idx| self.names[idx as usize].as_str())
            .collect()
    }

    /// Returns all timezone names in the dataset.
    #[must_use]
    pub fn timezonenames(&self) -> Vec<&str> {
        self.names.iter().map(String::as_str).collect()
    }

    /// Returns the dataset release this finder was built from (e.g. `2026c`).
    #[must_use]
    pub fn data_version(&self) -> &str {
        self.reader.data_version()
    }

    /// Converts all timezone boundaries to a GeoJSON FeatureCollection.
    ///
    /// Unlike [`DefaultFinder`], which exports polygons it already holds,
    /// this decodes the whole file's geometry on demand — roughly the cost of
    /// loading an expanded finder. A timezone that fails to decode is
    /// omitted; use [`get_tz_geojson`] when the error matters.
    ///
    /// [`get_tz_geojson`]: EmbeddedFinder::get_tz_geojson
    #[cfg(feature = "export-geojson")]
    #[cfg_attr(docsrs, doc(cfg(feature = "export-geojson")))]
    #[must_use]
    pub fn to_geojson(&self) -> BoundaryFile {
        let features = (0..self.names.len())
            .filter_map(|i| {
                let polys = self.reader.expand_timezone(i as u32).ok()?;
                Some(geojson::feature_from_expanded(
                    self.names[i].clone(),
                    &polys,
                ))
            })
            .collect();
        geojson::collection(features)
    }

    /// Converts one timezone's boundaries to a GeoJSON FeatureCollection,
    /// decoding only that timezone's rings. Returns `None` when the dataset
    /// does not contain the name or its geometry fails to decode.
    #[cfg(feature = "export-geojson")]
    #[cfg_attr(docsrs, doc(cfg(feature = "export-geojson")))]
    #[must_use]
    pub fn get_tz_geojson(&self, timezone_name: &str) -> Option<BoundaryFile> {
        let mut features = Vec::new();
        for (i, name) in self.names.iter().enumerate() {
            if name != timezone_name {
                continue;
            }
            let polys = self.reader.expand_timezone(i as u32).ok()?;
            features.push(geojson::feature_from_expanded(name.clone(), &polys));
        }
        if features.is_empty() {
            None
        } else {
            Some(geojson::collection(features))
        }
    }

    /// Converts one timezone's FUZZY preindex tiles to a GeoJSON
    /// FeatureCollection; see [`DefaultFinder::get_tz_preindex_geojson`].
    /// Results match [`DefaultFinder`] over the same file.
    #[cfg(feature = "export-geojson")]
    #[cfg_attr(docsrs, doc(cfg(feature = "export-geojson")))]
    #[must_use]
    pub fn get_tz_preindex_geojson(&self, timezone_name: &str) -> Option<BoundaryFile> {
        if !self.reader.has_fuzzy() {
            return None;
        }
        let indices: Vec<u16> = self
            .names
            .iter()
            .enumerate()
            .filter(|(_, name)| name.as_str() == timezone_name)
            .filter_map(|(i, _)| u16::try_from(i).ok())
            .collect();
        if indices.is_empty() {
            return None;
        }
        // The FUZZY key array is stored sorted, so the filtered keys keep the
        // coarsest-zoom-first order DefaultFinder produces by sorting.
        let entries = self.reader.fuzzy_entries().ok()?;
        let keys: Vec<u64> = entries
            .iter()
            .filter(|(_, idxs)| idxs.iter().any(|i| indices.contains(i)))
            .map(|(key, _)| *key)
            .collect();
        if keys.is_empty() {
            return None;
        }
        Some(geojson::collection(vec![geojson::feature_from_tile_keys(
            timezone_name.to_string(),
            &keys,
        )]))
    }

    /// Converts the whole FUZZY preindex to a GeoJSON FeatureCollection; see
    /// [`DefaultFinder::to_preindex_geojson`]. Results match
    /// [`DefaultFinder`] over the same file.
    #[cfg(feature = "export-geojson")]
    #[cfg_attr(docsrs, doc(cfg(feature = "export-geojson")))]
    #[must_use]
    pub fn to_preindex_geojson(&self) -> Option<BoundaryFile> {
        if !self.reader.has_fuzzy() {
            return None;
        }
        let entries = self.reader.fuzzy_entries().ok()?;
        let mut grouped: std::collections::HashMap<u16, Vec<u64>> =
            std::collections::HashMap::new();
        for (key, idxs) in &entries {
            for &idx in idxs {
                grouped.entry(idx).or_default().push(*key);
            }
        }
        let features = (0..self.names.len())
            .filter_map(|i| {
                let keys = grouped.remove(&u16::try_from(i).ok()?)?;
                Some(geojson::feature_from_tile_keys(self.names[i].clone(), &keys))
            })
            .collect();
        Some(geojson::collection(features))
    }
}

#[cfg(any(feature = "bundled", feature = "full"))]
impl Default for EmbeddedFinder {
    fn default() -> Self {
        Self::new()
    }
}

/// deg2num is used to convert longitude, latitude to [Slippy map tilenames]
/// under specific zoom level.
///
/// [Slippy map tilenames]: https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
///
/// Example:
///
/// ```rust
/// use tzf_rs::deg2num;
/// let ret = deg2num(116.3883, 39.9289, 7);
/// assert_eq!((105, 48), ret);
/// ```
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names
)]
pub fn deg2num(lng: f64, lat: f64, zoom: i64) -> (i64, i64) {
    let n = (1i64 << zoom) as f64;
    let lat_rad = lat.to_radians();
    let xtile = (lng / 360.0 + 0.5) * n;
    let ytile = (1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n;

    // Possible precision loss here
    (xtile as i64, ytile as i64)
}
