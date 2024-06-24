use std::collections::HashMap;

use euclid::approxeq::ApproxEq;
use euclid::{Transform2D, UnknownUnit, Vector2D};
use geo::Geometry;
use geo_rasterize::LabelBuilder;
use image::{ImageBuffer, LumaA, Pixel};
use itertools::Itertools;

use crate::export::data_2d_statistics::Data2d;
use crate::export::image::{PixelMappable, ToImage};
use crate::file_type::Shp;

pub type Identity = usize;

/// Represents all relevant groupings of `PolygonCollection`s, each with a
/// unique identity
#[derive(Default)]
pub struct GeometryUniverse {
	geometry: HashMap<Identity, Geometry>,
	pub(crate) max_identity: Identity,
}

pub struct GeometryMap {
	pub(crate) universe: GeometryUniverse,
	pub(crate) map: Data2d<Identity>,
}

pub trait ToGeometryUniverse {
	fn to_geometry_universe(&self) -> GeometryUniverse;
}

pub trait IntoGeometryMap {
	fn into_geometry_map(self, width: usize, height: usize) -> GeometryMap;
}

impl ToGeometryUniverse for Shp<f64> {
	fn to_geometry_universe(&self) -> GeometryUniverse {
		let mut reader = self.reader.borrow_mut();

		let mut universe = GeometryUniverse::default();

		let increment = 1usize;
		let mut identity = 0usize;
		let mut printed_keys = false;

		for shape_record in reader.iter_shapes_and_records() {
			let (shape, record) = shape_record.expect("Failed to get shape/record");
			universe.geometry.insert(
				identity,
				shape
					.try_into()
					.expect("Failed to convert shapefile::Polygon to geo::MultiPolygon"),
			);
			identity = identity + increment;

			if !printed_keys {
				for (key, value) in record.into_iter().sorted_by_key(|(k, v)| k.clone()) {
					println!("  {key} = {value:?}");
				}
				printed_keys = true;
			}
		}
		universe.max_identity = identity - increment;
		universe
	}
}

type Transform = Transform2D<f64, UnknownUnit, UnknownUnit>;

fn get_longitude_latitude_transform(width: usize, height: usize) -> Transform {
	let x_translate = 180.0;
	let y_translate = 90.0;

	let x_scale = width as f64 / 360.0;
	let y_scale = height as f64 / 180.0;

	Transform::identity()
		.then_translate(Vector2D::new(x_translate, y_translate))
		.then_scale(x_scale, y_scale)
}

impl IntoGeometryMap for GeometryUniverse {
	fn into_geometry_map(self, width: usize, height: usize) -> GeometryMap {
		let transform = get_longitude_latitude_transform(width, height);

		let mut rasterizer = LabelBuilder::<Identity>::background(0u8.into())
			.width(width)
			.height(height)
			.geo_to_pix(transform)
			.build()
			.expect("Failed to build rasterizer");

		for (identity, polygon) in self.geometry.iter() {
			rasterizer.rasterize(polygon, *identity).expect(
				format!("Failed to rasterize Polygon with identity {identity:?}: {polygon:?}")
					.as_str(),
			)
		}

		let pixels = rasterizer.finish();
		let map = pixels.view().into();
		GeometryMap { map, universe: self }
	}
}

impl PixelMappable<Identity> for GeometryMap {
	fn get_pixel_map(&self) -> Box<dyn Fn(&Identity) -> u8> {
		assert!(self.universe.max_identity < 256);

		let range = self.universe.max_identity as f64;
		Box::new(move |value: &Identity| {
			let val_f = *value as f64;
			let portion = val_f / range;
			(255.0 * portion) as u8
		})
	}
}

impl ToImage<LumaA<u8>> for GeometryMap {
	type Data = Identity;

	fn width(&self) -> usize { self.map.width() }

	fn height(&self) -> usize { self.map.height() }

	fn to_image(&self) -> ImageBuffer<LumaA<u8>, Vec<<LumaA<u8> as Pixel>::Subpixel>> {
		const NUM_CHANNELS: usize = 2;

		let pixel_map = PixelMappable::<Identity>::get_pixel_map(self);
		let mut output_buffer = Vec::with_capacity(self.width() * self.height() * NUM_CHANNELS);

		let outline_kernel = [[0.0, 0.2, 0.0], [0.2, 0.2, 0.2], [0.0, 0.2, 0.0]];

		for (row_index, row) in self.map.rows.iter().enumerate().rev() {
			for (col_index, val) in row.columns.iter().enumerate() {
				let within_country =
					self.sum_kernel_to_center(&outline_kernel, col_index, row_index);
				if within_country {
					output_buffer.push(0u8);
				} else {
					output_buffer.push(255u8);
				}
				output_buffer.push(pixel_map(val));
			}
		}

		ImageBuffer::from_raw(self.width() as u32, self.height() as u32, output_buffer)
			.expect("Failed to create image!")
	}
}

impl GeometryMap {
	fn sum_kernel_to_center<const N: usize>(
		&self,
		kernel: &[[f32; N]; N],
		center_x: usize,
		center_y: usize,
	) -> bool {
		let center_value = self.map.rows[center_y].columns[center_x];
		let kernel_radius = (N - 1) / 2;
		let kernel_start_y = center_y.saturating_sub(kernel_radius);
		let kernel_start_x = center_x.saturating_sub(kernel_radius);

		let mut sum = 0.0;

		for (map_row, kernel_row) in
			self.map.rows.iter().skip(kernel_start_y).take(N).zip(kernel.iter())
		{
			for (map_val, kernel_val) in
				map_row.columns.iter().skip(kernel_start_x).take(N).zip(kernel_row.iter())
			{
				sum += (*map_val as f32) * *kernel_val;
			}
		}
		let eps: f32 = 0.1 / 255.0;
		sum.approx_eq_eps(&(center_value as f32), &eps)
	}
}
