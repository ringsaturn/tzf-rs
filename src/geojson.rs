//! GeoJSON export types and conversions (feature `export-geojson`).
//!
//! Coordinates are converted from the 1e5-scaled int32 storage through `f32`,
//! matching the precision of the v1 export path, so output stays comparable
//! across releases.

use crate::finder::Item;
use crate::tzb::{ExpandedPolygon, TileId};
use serde::{Deserialize, Serialize};

pub type PolygonCoordinates = Vec<Vec<[f64; 2]>>;
pub type MultiPolygonCoordinates = Vec<PolygonCoordinates>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryDefine {
    #[serde(rename = "type")]
    pub geometry_type: String,
    pub coordinates: MultiPolygonCoordinates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertiesDefine {
    pub tzid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureItem {
    #[serde(rename = "type")]
    pub feature_type: String,
    pub properties: PropertiesDefine,
    pub geometry: GeometryDefine,
}

impl FeatureItem {
    /// Serializes to a JSON string. Kept as an inherent method for v1 API
    /// compatibility.
    #[allow(clippy::inherent_to_string)]
    #[must_use]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    #[must_use]
    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryFile {
    #[serde(rename = "type")]
    pub collection_type: String,
    pub features: Vec<FeatureItem>,
}

impl BoundaryFile {
    /// Serializes to a JSON string. Kept as an inherent method for v1 API
    /// compatibility.
    #[allow(clippy::inherent_to_string)]
    #[must_use]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    #[must_use]
    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

pub(crate) fn collection(features: Vec<FeatureItem>) -> BoundaryFile {
    BoundaryFile {
        collection_type: "FeatureCollection".to_string(),
        features,
    }
}

fn scaled_coord(v: i32) -> f64 {
    #[allow(clippy::cast_possible_truncation)]
    let narrowed = (f64::from(v) / 1e5) as f32;
    f64::from(narrowed)
}

fn ring_coords<'a>(points: impl Iterator<Item = &'a geometry_rs::Point<i32>>) -> Vec<[f64; 2]> {
    points
        .map(|p| [scaled_coord(p.x), scaled_coord(p.y)])
        .collect()
}

/// Like [`ring_coords`], but for the open rings the `.tzb` expansion yields:
/// GeoJSON rings must be closed, so the first coordinate is repeated at the
/// end (matching the closed storage the materialized finder exports).
fn ring_coords_closed<'a>(
    points: impl Iterator<Item = &'a geometry_rs::Point<i32>>,
) -> Vec<[f64; 2]> {
    let mut coords = ring_coords(points);
    if let Some(&first) = coords.first() {
        coords.push(first);
    }
    coords
}

/// Builds one GeoJSON Feature from a finder item's materialized polygons.
pub(crate) fn feature_from_item(item: &Item) -> FeatureItem {
    let coordinates = item
        .polys
        .iter()
        .map(|poly| {
            let mut rings = PolygonCoordinates::new();
            rings.push(ring_coords(poly.exterior().iter()));
            for hole in poly.holes() {
                rings.push(ring_coords(hole.iter()));
            }
            rings
        })
        .collect();
    feature(item.name.clone(), coordinates)
}

/// Builds one GeoJSON Feature from rings expanded out of a `.tzb` file.
pub(crate) fn feature_from_expanded(name: String, polys: &[ExpandedPolygon]) -> FeatureItem {
    let coordinates = polys
        .iter()
        .map(|poly| {
            let mut rings = PolygonCoordinates::new();
            rings.push(ring_coords_closed(poly.exterior.iter()));
            for hole in &poly.holes {
                rings.push(ring_coords_closed(hole.iter()));
            }
            rings
        })
        .collect();
    feature(name, coordinates)
}

/// Builds one GeoJSON Feature from FUZZY preindex tile keys: a MultiPolygon
/// holding each tile's bounding rectangle as one closed ring.
pub(crate) fn feature_from_tile_keys(name: String, keys: &[u64]) -> FeatureItem {
    let coordinates = keys.iter().map(|&key| vec![TileId(key).polygon()]).collect();
    feature(name, coordinates)
}

fn feature(name: String, coordinates: MultiPolygonCoordinates) -> FeatureItem {
    FeatureItem {
        feature_type: "Feature".to_string(),
        properties: PropertiesDefine { tzid: name },
        geometry: GeometryDefine {
            geometry_type: "MultiPolygon".to_string(),
            coordinates,
        },
    }
}
