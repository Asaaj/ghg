use image::{
	GrayAlphaImage, GrayImage, ImageBuffer, Luma, LumaA, Pixel, Rgb, RgbImage, Rgba, RgbaImage,
};
use itertools::izip;
#[cfg(feature = "read_shapefile")]
use shapefile::Shape;

use crate::export::data_2d_statistics::{Data2dStatistics, DataType};
#[cfg(feature = "read_shapefile")]
use crate::export::geometry_map::{GeometryMap, Identity};
#[cfg(feature = "read_shapefile")]
use crate::file_type::Shp;

pub trait ToImage<P: Pixel> {
	type Data;
	fn width(&self) -> usize;
	fn height(&self) -> usize;
	fn to_image(&self) -> ImageBuffer<P, Vec<P::Subpixel>>;
}

pub trait PixelMappable<T> {
	fn get_pixel_map(&self) -> Box<dyn Fn(&T) -> u8>;
}

impl PixelMappable<f64> for Data2dStatistics<f64> {
	fn get_pixel_map(&self) -> Box<dyn Fn(&f64) -> u8> {
		let range = self.max.unwrap() - self.min.unwrap();
		let offset = self.min.unwrap();
		Box::new(move |value: &f64| {
			let portion = (*value - offset) / range;
			(255.0 * portion) as u8
		})
	}
}

impl<T: DataType> ToImage<Luma<u8>> for Data2dStatistics<T>
where
	Data2dStatistics<T>: PixelMappable<T>,
{
	type Data = T;

	fn width(&self) -> usize { self.data.width() }

	fn height(&self) -> usize { self.data.height() }

	fn to_image(&self) -> ImageBuffer<Luma<u8>, Vec<u8>> {
		let pixel_map = PixelMappable::<T>::get_pixel_map(self);
		let mut output_buffer = Vec::with_capacity(self.width() * self.height());
		for row in self.data.rows.iter().rev() {
			for val in row.columns.iter() {
				output_buffer.push(pixel_map(val));
			}
		}

		GrayImage::from_raw(self.width() as u32, self.height() as u32, output_buffer)
			.expect("Failed to create image!")
	}
}

impl<T: DataType> ToImage<Luma<u8>> for [Data2dStatistics<T>; 1]
where
	Data2dStatistics<T>: PixelMappable<T>,
{
	type Data = T;

	fn width(&self) -> usize { self[0].width() }

	fn height(&self) -> usize { self[0].height() }

	fn to_image(&self) -> ImageBuffer<Luma<u8>, Vec<u8>> { return self[0].to_image() }
}

impl<T: DataType> ToImage<LumaA<u8>> for [Data2dStatistics<T>; 2]
where
	Data2dStatistics<T>: PixelMappable<T>,
{
	type Data = T;

	fn width(&self) -> usize {
		assert_eq!(self[0].width(), self[1].width());
		self[0].width()
	}

	fn height(&self) -> usize {
		assert_eq!(self[0].height(), self[1].height());
		self[0].height()
	}

	fn to_image(&self) -> ImageBuffer<LumaA<u8>, Vec<u8>> {
		let pixel_maps: Vec<Box<dyn Fn(&T) -> u8>> =
			self.iter().map(|ds| PixelMappable::<T>::get_pixel_map(ds)).collect();
		let mut output_buffer = Vec::with_capacity(2 * self.width() * self.height());
		for (row1, row2) in self[0].data.rows.iter().rev().zip(self[1].data.rows.iter().rev()) {
			for (val0, val1) in row1.columns.iter().zip(row2.columns.iter()) {
				output_buffer.push(pixel_maps[0](val0));
				output_buffer.push(pixel_maps[1](val1));
			}
		}

		GrayAlphaImage::from_raw(self.width() as u32, self.height() as u32, output_buffer)
			.expect("Failed to create image!")
	}
}

impl<T: DataType> ToImage<Rgb<u8>> for [Data2dStatistics<T>; 3]
where
	Data2dStatistics<T>: PixelMappable<T>,
{
	type Data = T;

	fn width(&self) -> usize {
		assert_eq!(self[0].width(), self[1].width());
		assert_eq!(self[0].width(), self[2].width());
		self[0].width()
	}

	fn height(&self) -> usize {
		assert_eq!(self[0].height(), self[1].height());
		assert_eq!(self[0].height(), self[2].height());
		self[0].height()
	}

	fn to_image(&self) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
		let pixel_maps: Vec<Box<dyn Fn(&T) -> u8>> =
			self.iter().map(|ds| PixelMappable::<T>::get_pixel_map(ds)).collect();
		let mut output_buffer = Vec::with_capacity(3 * self.width() * self.height());
		for rows in izip!(
			self[0].data.rows.iter().rev(),
			self[1].data.rows.iter().rev(),
			self[2].data.rows.iter().rev()
		) {
			for vals in izip!(rows.0.columns.iter(), rows.1.columns.iter(), rows.2.columns.iter()) {
				output_buffer.push(pixel_maps[0](vals.0));
				output_buffer.push(pixel_maps[1](vals.1));
				output_buffer.push(pixel_maps[2](vals.2));
			}
		}

		RgbImage::from_raw(self.width() as u32, self.height() as u32, output_buffer)
			.expect("Failed to create image!")
	}
}

impl<T: DataType> ToImage<Rgba<u8>> for [Data2dStatistics<T>; 4]
where
	Data2dStatistics<T>: PixelMappable<T>,
{
	type Data = T;

	fn width(&self) -> usize {
		assert_eq!(self[0].width(), self[1].width());
		assert_eq!(self[0].width(), self[2].width());
		assert_eq!(self[0].width(), self[3].width());
		self[0].width()
	}

	fn height(&self) -> usize {
		assert_eq!(self[0].height(), self[1].height());
		assert_eq!(self[0].height(), self[2].height());
		assert_eq!(self[0].height(), self[3].height());
		self[0].height()
	}

	fn to_image(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
		let pixel_maps: Vec<Box<dyn Fn(&T) -> u8>> =
			self.iter().map(|ds| PixelMappable::<T>::get_pixel_map(ds)).collect();
		let mut output_buffer = Vec::with_capacity(3 * self.width() * self.height());
		for rows in izip!(
			self[0].data.rows.iter().rev(),
			self[1].data.rows.iter().rev(),
			self[2].data.rows.iter().rev(),
			self[3].data.rows.iter().rev(),
		) {
			for vals in izip!(
				rows.0.columns.iter(),
				rows.1.columns.iter(),
				rows.2.columns.iter(),
				rows.3.columns.iter()
			) {
				output_buffer.push(pixel_maps[0](vals.0));
				output_buffer.push(pixel_maps[1](vals.1));
				output_buffer.push(pixel_maps[2](vals.2));
				output_buffer.push(pixel_maps[3](vals.3));
			}
		}

		RgbaImage::from_raw(self.width() as u32, self.height() as u32, output_buffer)
			.expect("Failed to create image!")
	}
}

#[cfg(feature = "read_shapefile")]
impl ToImage<Luma<u8>> for Shp<f64> {
	type Data = f64;

	fn width(&self) -> usize { self.width }

	fn height(&self) -> usize { self.height }

	fn to_image(&self) -> ImageBuffer<Luma<u8>, Vec<<Luma<u8> as Pixel>::Subpixel>> {
		let mut reader = self.reader.borrow_mut();

		let buffer_length = self.width() * self.height();
		let mut output_buffer = vec![0; buffer_length];

		let mut num_shapes = 0;
		for shape_record in reader.iter_shapes_and_records() {
			let (shape, record) = shape_record.expect("Failed to get shape/record");
			num_shapes += 1;
			match shape {
				Shape::Polygon(polygon) => {
					for ring in polygon.rings() {
						for point in ring.points() {
							let (x, y) = self.filter_coordinates(point.x, point.y);
							self.set_pixel(&mut output_buffer, 255u8, x, y, 0);
						}
					}
				}
				other => println!("Unsupported shape type: {}", other),
			};
		}

		println!("Total: {} polygons", num_shapes);

		GrayImage::from_raw(self.width() as u32, self.height() as u32, output_buffer)
			.expect("Failed to create image!")
	}
}
