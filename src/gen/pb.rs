#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Point {
    #[prost(float, tag = "1")]
    pub lng: f32,
    #[prost(float, tag = "2")]
    pub lat: f32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Polygon {
    #[prost(message, repeated, tag = "1")]
    pub points: ::prost::alloc::vec::Vec<Point>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Timezone {
    #[prost(message, repeated, tag = "1")]
    pub polygons: ::prost::alloc::vec::Vec<Polygon>,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Timezones {
    #[prost(message, repeated, tag = "1")]
    pub timezones: ::prost::alloc::vec::Vec<Timezone>,
}
