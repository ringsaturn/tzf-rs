use tzf_rs::DefaultFinder;

fn main() {
    let default_finder = DefaultFinder::new();
    let lng = 139.6917;
    let lat = 35.6895;

    let tz_name = default_finder.get_tz_name(lng, lat).to_owned();
    println!(
        "The timezone at longitude {}, latitude {} is: {}",
        lng, lat, tz_name
    );

    // Get the Polygon boundary for the timezone
    if let Some(boundary_file) = default_finder.finder.get_tz_geojson(&tz_name) {
        // It's GeoJSON Feature Collection, and the features contains "MultiPolygon" geometry for the timezone.
        println!("Found GeoJSON feature for timezone: {}", tz_name);
        let mut polygons: usize = 0;
        for feature in boundary_file.features {
            polygons += feature.geometry.coordinates.len();
        }
        println!(
            "Total number of polygons in feature collection: {}",
            polygons
        );
    }

    // Get the Index polygon boundary for the timezone
    if let Some(index_boundary_file) = default_finder.fuzzy_finder.get_tz_geojson(&tz_name) {
        // It's GeoJSON Feature, and the geometry contains "MultiPolygon" for the timezone index.
        // But the Polygons are actually map tiles.
        println!("Found Index GeoJSON feature for timezone: {}", tz_name);
        let mut polygons: usize = 0;
        for polygon in index_boundary_file.geometry.coordinates {
            polygons += polygon.len();
        }
        println!(
            "Total number of tile polygons in index feature: {}",
            polygons
        );
    }
}
