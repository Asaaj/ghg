use std::env;
use std::path::Path;

use ghg_data_processing::export::geometry_map::{
	GeometryUniverse, IntoGeometryMap, ToGeometryUniverse,
};
use ghg_data_processing::export::image::ToImage;
use ghg_data_processing::file_type::{DataFile, ShapefileMetadata, Shp};
use ghg_data_processing::read_data::find_data_files;

fn main() -> std::io::Result<()> {
	let args: Vec<String> = env::args().collect();
	assert_eq!(args.len(), 2);

	let output_root = Path::new("ghg/www/images/countries");
	assert!(output_root.exists());

	let data_source = Path::new(&args[1]);

	let data_files = find_data_files(data_source, &[Shp::<f64>::extension()]);
	if data_files.len() == 0 {
		println!("No data files found in path {:?}", data_source);
		return Ok(());
	}

	println!("Found files in path {:?}", data_source);
	for file in &data_files {
		println!("  - {file:?}");
	}

	// let metadata = ShapefileMetadata { width: 200, height: 100 };
	let metadata = ShapefileMetadata { width: 10800, height: 5400 };
	// let metadata = ShapefileMetadata { width: 21600, height: 10800 };

	for file in &data_files {
		let geometry_universe: GeometryUniverse = Shp::<f64>::open(file, metadata)
			.expect(format!("Failed to read file {:?}", file.file_name().unwrap()).as_str())
			.to_geometry_universe();

		let geometry_map = geometry_universe.into_geometry_map(metadata.width, metadata.height);
		let image = geometry_map.to_image();

		let output_name = output_root.join("country_map.png");
		image.save(output_name).expect("Failed to save image data");
	}

	Ok(())
}
