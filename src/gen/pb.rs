/// Basic Point data define.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Point {
    #[prost(float, tag = "1")]
    pub lng: f32,
    #[prost(float, tag = "2")]
    pub lat: f32,
}
/// Define a polygon, mostly based on GeoJSON's Polygon define.
///
/// Excerpt from RFC-9476 section 'Polygon'
///
///    -  A linear ring is a closed LineString with four or more positions.
///    -  The first and last positions are equivalent, and they MUST contain
///      identical values; their representation SHOULD also be identical.
///    -  A linear ring is the boundary of a surface or the boundary of a
///      hole in a surface.
///    -  A linear ring MUST follow the right-hand rule with respect to the
///      area it bounds, i.e., exterior rings are counterclockwise, and
///      holes are clockwise.
///
///    Note: the \[GJ2008\] specification did not discuss linear ring winding
///    order.  For backwards compatibility, parsers SHOULD NOT reject
///    Polygons that do not follow the right-hand rule.
///
///    Though a linear ring is not explicitly represented as a GeoJSON
///    geometry type, it leads to a canonical formulation of the Polygon
///    geometry type definition as follows:
///
///    -  For type "Polygon", the "coordinates" member MUST be an array of
///      linear ring coordinate arrays.
///    -  For Polygons with more than one of these rings, the first MUST be
///      the exterior ring, and any others MUST be interior rings.  The
///      exterior ring bounds the surface, and the interior rings (if
///      present) bound holes within the surface.
///
/// \[GJ2008\]: <https://geojson.org/geojson-spec>
///
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Polygon {
    /// define the "exterior ring"
    #[prost(message, repeated, tag = "1")]
    pub points: ::prost::alloc::vec::Vec<Point>,
    /// define the "interior rings" as holes
    #[prost(message, repeated, tag = "2")]
    pub holes: ::prost::alloc::vec::Vec<Polygon>,
}
/// Timezone is a timezone's all data.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Timezone {
    #[prost(message, repeated, tag = "1")]
    pub polygons: ::prost::alloc::vec::Vec<Polygon>,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Timezones {
    #[prost(message, repeated, tag = "1")]
    pub timezones: ::prost::alloc::vec::Vec<Timezone>,
    /// Reduced data will toggle neighbor search as plan b
    #[prost(bool, tag = "2")]
    pub reduced: bool,
    #[prost(string, tag = "3")]
    pub version: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CompressedPolygon {
    #[prost(bytes = "vec", tag = "1")]
    pub points: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag = "2")]
    pub holes: ::prost::alloc::vec::Vec<CompressedPolygon>,
}
/// CompressedTimezonesItem designed for binary file as small as possible.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CompressedTimezone {
    #[prost(message, repeated, tag = "1")]
    pub data: ::prost::alloc::vec::Vec<CompressedPolygon>,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CompressedTimezones {
    #[prost(enumeration = "CompressMethod", tag = "1")]
    pub method: i32,
    #[prost(message, repeated, tag = "2")]
    pub timezones: ::prost::alloc::vec::Vec<CompressedTimezone>,
    #[prost(string, tag = "3")]
    pub version: ::prost::alloc::string::String,
}
/// PreindexTimezone tile item.
///
/// The X/Y/Z are OSM style like map tile index values.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PreindexTimezone {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(int32, tag = "2")]
    pub x: i32,
    #[prost(int32, tag = "3")]
    pub y: i32,
    #[prost(int32, tag = "4")]
    pub z: i32,
}
/// PreindexTimezones is all preindex timezone's dumps.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PreindexTimezones {
    /// which zoom value the tiles generated
    #[prost(int32, tag = "1")]
    pub idx_zoom: i32,
    /// which zoom value the tiles merge up with.
    #[prost(int32, tag = "2")]
    pub agg_zoom: i32,
    #[prost(message, repeated, tag = "3")]
    pub keys: ::prost::alloc::vec::Vec<PreindexTimezone>,
    #[prost(string, tag = "4")]
    pub version: ::prost::alloc::string::String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum CompressMethod {
    Unknown = 0,
    /// <https://developers.google.com/maps/documentation/utilities/polylinealgorithm>
    Polyline = 1,
}
impl CompressMethod {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            CompressMethod::Unknown => "Unknown",
            CompressMethod::Polyline => "Polyline",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "Unknown" => Some(Self::Unknown),
            "Polyline" => Some(Self::Polyline),
            _ => None,
        }
    }
}
