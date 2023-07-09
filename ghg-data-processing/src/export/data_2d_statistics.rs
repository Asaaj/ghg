use std::ops::Sub;

use ghg_data_core::metadata::{ChannelMetadata, Metadata};
use ndarray::{ArrayView1, ArrayView2};

pub trait DataType = Copy + Clone + Default + PartialOrd + Sub<Output = Self>;

#[derive(Clone)]
pub struct Data1d<T> {
	pub columns: Vec<T>,
}

impl<T: Clone + Default> Data1d<T> {
	fn new(size: usize) -> Self {
		let mut columns = Vec::with_capacity(size);
		columns.resize(size, T::default());
		Self { columns }
	}

	fn width(&self) -> usize { self.columns.len() }
}

impl<T: Clone> From<ArrayView1<'_, T>> for Data1d<T> {
	fn from(value: ArrayView1<'_, T>) -> Self { Self { columns: value.to_vec() } }
}

#[derive(Clone)]
pub struct Data2d<T> {
	pub rows: Vec<Data1d<T>>,
}

impl<T: Clone + Default> Data2d<T> {
	pub fn new(width: usize, height: usize) -> Self {
		let mut rows = Vec::with_capacity(height);
		rows.resize(height, Data1d::new(width));
		Self { rows }
	}

	pub fn width(&self) -> usize {
		if self.rows.len() == 0 {
			return 0;
		}
		self.rows[0].width()
	}

	pub fn height(&self) -> usize { self.rows.len() }
}

impl<T: Clone> From<ArrayView2<'_, T>> for Data2d<T> {
	fn from(value: ArrayView2<'_, T>) -> Self {
		Self { rows: value.rows().into_iter().map(|r| r.into()).collect() }
	}
}

#[derive(Clone)]
pub struct Data2dStatistics<T: DataType> {
	pub name: String,
	pub data: Data2d<T>,
	pub min: Option<T>,
	pub max: Option<T>,
}

impl<T: DataType> Sub for &Data2dStatistics<T>
where
	T: Sub<Output = T>,
{
	type Output = Data2dStatistics<T>;

	fn sub(self, rhs: Self) -> Self::Output {
		let mut difference = Data2d::new(self.data.width(), self.data.height());
		let mut min = None;
		let mut max = None;

		for (row, (a_row, b_row)) in self.data.rows.iter().zip(rhs.data.rows.iter()).enumerate() {
			for (col, (a_val, b_val)) in a_row.columns.iter().zip(b_row.columns.iter()).enumerate()
			{
				let val_difference = *b_val - *a_val;
				if max.is_none() || val_difference > max.unwrap() {
					max = Some(val_difference);
				}
				if min.is_none() || val_difference < min.unwrap() {
					min = Some(val_difference);
				}
				difference.rows[row].columns[col] = val_difference;
			}
		}

		Data2dStatistics {
			name: format!("{} - {}", self.name, rhs.name),
			data: difference,
			min,
			max,
		}
	}
}

pub trait ToMetadata {
	fn to_metadata(&self) -> Metadata;
}

impl ToMetadata for [Data2dStatistics<f64>; 3] {
	fn to_metadata(&self) -> Metadata {
		self.iter()
			.map(|ds| ChannelMetadata { min: ds.min.unwrap(), max: ds.max.unwrap() })
			.collect()
	}
}

impl ToMetadata for [Data2dStatistics<f64>; 4] {
	fn to_metadata(&self) -> Metadata {
		self.iter()
			.map(|ds| ChannelMetadata { min: ds.min.unwrap(), max: ds.max.unwrap() })
			.collect()
	}
}
