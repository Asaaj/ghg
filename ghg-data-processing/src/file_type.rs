use std::cell::RefCell;
use std::default::Default;
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;
use std::marker::PhantomData;
use std::path::Path;

use crate::export::data_2d_statistics::{Data2dStatistics, DataType};

pub trait VariableDescriptor = Clone;

pub trait Metadata = Clone;

pub trait DataFile<TData: DataType, TMetadata: Metadata> {
	fn extension() -> &'static OsStr;
	fn open(path: &Path, metadata: TMetadata) -> Result<Self, String>
	where
		Self: Sized;
}

pub trait ToStatistics<TData: DataType, TVar: VariableDescriptor> {
	fn read_variables(&self, variables: &[TVar]) -> Vec<Data2dStatistics<TData>>;
}

/// *.shp + *.shx + *.dbf files (a.k.a. Shapefiles)
#[cfg(feature = "read_shapefile")]
pub struct Shp<T: DataType> {
	pub(crate) reader: RefCell<shapefile::Reader<BufReader<File>, BufReader<File>>>,
	pub(crate) width: usize,
	pub(crate) height: usize,
	phantom: PhantomData<T>,
}

#[cfg(feature = "read_shapefile")]
#[derive(Copy, Clone, Debug)]
pub struct ShapefileMetadata {
	pub width: usize,
	pub height: usize,
}

#[cfg(feature = "read_shapefile")]
impl<T: DataType> DataFile<T, ShapefileMetadata> for Shp<T> {
	fn extension() -> &'static OsStr { OsStr::new("shp") }

	fn open(path: &Path, metadata: ShapefileMetadata) -> Result<Self, String>
	where
		Self: Sized,
	{
		Ok(Self {
			reader: RefCell::new(shapefile::Reader::from_path(path).map_err(|e| e.to_string())?),
			width: metadata.width,
			height: metadata.height,
			phantom: PhantomData::default(),
		})
	}
}

#[cfg(feature = "read_shapefile")]
impl Shp<f64> {
	const NUM_CHANNELS: usize = 1;

	pub(crate) fn filter_coordinates(&self, x: f64, y: f64) -> (usize, usize) {
		let translated_x = x + 180.0;
		let translated_y = (-y) + 90.0;
		let filtered_x =
			((translated_x * (self.width as f64) / 360.0) as usize).clamp(0, self.width - 1);
		let filtered_y =
			((translated_y * (self.height as f64) / 180.0) as usize).clamp(0, self.height - 1);
		(filtered_x, filtered_y)
	}

	pub(crate) fn set_pixel(
		&self,
		buffer: &mut Vec<u8>,
		value: u8,
		x: usize,
		y: usize,
		channel: usize,
	) {
		let one_channel_coords = y * self.width + x;
		let multi_channel_coords = one_channel_coords * Shp::NUM_CHANNELS;
		let this_channel_coords = multi_channel_coords + channel;
		buffer[this_channel_coords] = value;
	}
}

#[cfg(feature = "read_netcdf")]
pub mod cdf {
	use super::*;
	use crate::export::data_2d_statistics::Data2d;

	#[derive(Debug)]
	/// Shared implementation of a netcdf-readable file
	struct CdfReadableData<T: DataType> {
		path: String,
		contents: netcdf::File,
		metadata: CdfMetadata,
		t: PhantomData<T>,
	}

	#[derive(Copy, Clone, Debug)]
	pub struct CdfMetadata {
		pub width_dimension: usize,
		pub height_dimension: usize,
	}

	#[derive(Debug)]
	/// *.nc files
	pub struct Nc<T: DataType> {
		data: CdfReadableData<T>,
	}

	impl<T: DataType + netcdf::NcPutGet> DataFile<T, CdfMetadata> for Nc<T> {
		fn extension() -> &'static OsStr { OsStr::new("nc") }

		fn open(path: &Path, metadata: CdfMetadata) -> Result<Self, String>
		where
			Self: Sized,
		{
			match CdfReadableData::open(path, metadata) {
				Ok(data) => Ok(Self { data }),
				Err(error) => Err(error),
			}
		}
	}

	impl<T: DataType + netcdf::NcPutGet> ToStatistics<T, String> for Nc<T> {
		fn read_variables(&self, variables: &[String]) -> Vec<Data2dStatistics<T>> {
			self.data.read_variables(variables)
		}
	}

	#[derive(Debug)]
	/// *.nc4 files
	pub struct Nc4<T: DataType> {
		data: CdfReadableData<T>,
	}

	impl<T: DataType + netcdf::NcPutGet> DataFile<T, CdfMetadata> for Nc4<T> {
		fn extension() -> &'static OsStr { OsStr::new("nc4") }

		fn open(path: &Path, metadata: CdfMetadata) -> Result<Self, String>
		where
			Self: Sized,
		{
			match CdfReadableData::open(path, metadata) {
				Ok(data) => Ok(Self { data }),
				Err(error) => Err(error),
			}
		}
	}

	impl<T: DataType + netcdf::NcPutGet> ToStatistics<T, String> for Nc4<T> {
		fn read_variables(&self, variables: &[String]) -> Vec<Data2dStatistics<T>> {
			self.data.read_variables(variables)
		}
	}

	impl<T: DataType + netcdf::NcPutGet> CdfReadableData<T> {
		fn open(path: &Path, metadata: CdfMetadata) -> Result<Self, String> {
			let path_str: String = path.to_str().unwrap().to_owned();
			match netcdf::open(path) {
				Ok(contents) => {
					Ok(Self { path: path_str, contents, metadata, t: Default::default() })
				}
				Err(error) => Err(format!("CDF open error occurred: {error:?}")),
			}
		}

		fn read_variables(&self, variables: &[String]) -> Vec<Data2dStatistics<T>> {
			println!("Reading data from {:?}. Variables: {:?}", self.path, variables);
			let mut all_data = Vec::new();

			if variables.len() > 0 {
				for name in variables {
					if let Some(v) = self.contents.variable(name.as_str()) {
						all_data.push(self.read_variable(&v))
					} else {
						panic!("Unknown variable {name} in file {:?}", self.path)
					}
				}
				assert_eq!(all_data.len(), variables.len());
			} else {
				println!("No variables specified; reading all available variables");
				for v in self.contents.variables() {
					all_data.push(self.read_variable(&v))
				}
			}

			all_data
		}
	}

	impl<T: DataType + netcdf::NcPutGet> CdfReadableData<T> {
		fn read_variable(&self, v: &netcdf::Variable) -> Data2dStatistics<T> {
			println!("Reading variable: {:?} (length = {})", v.name(), v.len());

			let dim = v.dimensions();
			if dim.len() >= 2 {
				return self.read_2d_variable(v);
			} else {
				return self.read_1d_variable(v);
			}
		}

		fn read_2d_variable(&self, v: &netcdf::Variable) -> Data2dStatistics<T> {
			let dim = v.dimensions();
			assert!(dim.len() >= 2);

			let mut indices = vec![0; dim.len()];

			let height = dim[self.metadata.height_dimension].len();
			let width = dim[self.metadata.width_dimension].len();

			let mut data = Data2d::<T>::new(width, height);
			let mut min = None;
			let mut max = None;

			let mut valid: usize = 0;
			for row in 0..height {
				for column in 0..width {
					indices[self.metadata.height_dimension] = row;
					indices[self.metadata.width_dimension] = column;
					if let Some((val, new_min, new_max)) = Self::stat_cell(v, &indices, min, max) {
						data.rows[row].columns[column] = val;
						min = new_min;
						max = new_max;
						valid += 1;
					}
				}
			}

			assert_eq!(valid, data.height() * data.width());

			Data2dStatistics { name: v.name(), data, min, max }
		}

		fn read_1d_variable(&self, v: &netcdf::Variable) -> Data2dStatistics<T> {
			let dim = v.dimensions();
			let width = dim[self.metadata.width_dimension].len();
			let variable_name = v.name();
			assert!(
				width > 0,
				"{}",
				format!("Invalid width for 1D variable {variable_name}: {width}")
			);

			let mut data = Data2d::<T>::new(width, 1);
			let mut min = None;
			let mut max = None;

			let mut valid: usize = 0;
			for column in 0..width {
				if let Some((val, new_min, new_max)) = Self::stat_cell(v, &[column], min, max) {
					data.rows[0].columns[column] = val;
					min = new_min;
					max = new_max;
					valid += 1;
				}
			}

			assert_eq!(valid, data.width());

			Data2dStatistics { name: v.name(), data, min, max }
		}

		fn stat_cell(
			v: &netcdf::Variable,
			indices: &[usize],
			mut min: Option<T>,
			mut max: Option<T>,
		) -> Option<(T, Option<T>, Option<T>)> {
			if let Ok(val) = v.value::<T, &[usize]>(indices) {
				if max.is_none() || val > max.unwrap() {
					max = Some(val);
				}
				if min.is_none() || val < min.unwrap() {
					min = Some(val);
				}
				return Some((val, min, max));
			}
			return None;
		}
	}
}
