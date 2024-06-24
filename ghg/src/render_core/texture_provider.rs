use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::utils::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct TextureProvider {
	next_index: Arc<AtomicU32>,
}

impl TextureProvider {
	const MAX_TEXTURES: u32 = 16;

	// WebGL2 minimum number frag textures required on all platforms

	pub fn take(&mut self) -> u32 {
		let result = self.next_index.fetch_add(1, Ordering::Relaxed);
		if result >= Self::MAX_TEXTURES {
			ghg_error!("Cannot guarantee so many textures! Allocated {}", result + 1);
		}
		result
	}
}
