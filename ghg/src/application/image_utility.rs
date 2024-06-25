use wasm_bindgen::JsValue;
use wasm_bindgen::__rt::IntoJsResult;
use web_sys::WebGl2RenderingContext;

const MAX_MIPMAP_LEVEL: usize = 10;

pub fn biggest_mipmap_level(
	context: WebGl2RenderingContext,
	level_0_max_size: usize,
) -> Result<usize, JsValue> {
	let max_texture = context
		.get_parameter(WebGl2RenderingContext::MAX_TEXTURE_SIZE)?
		.as_f64()
		.ok_or(JsValue::from_str("Max texture size is not a float"))?;

	compute_biggest_level(max_texture as usize, level_0_max_size)
		.ok_or(format!("No mipmap level found before {MAX_MIPMAP_LEVEL}").into_js_result().unwrap())
}

fn compute_biggest_level(max_texture: usize, level_0_max_size: usize) -> Option<usize> {
	for attempted_level in 0..MAX_MIPMAP_LEVEL {
		let level_dimension = level_0_max_size / (1 << attempted_level);
		if level_dimension <= max_texture {
			return Some(attempted_level);
		}
	}
	return None;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_max_larger_than_level_0() {
		let result = compute_biggest_level(100, 90);
		assert_eq!(result, Some(0));
	}

	#[test]
	fn test_max_between_levels_0_and_1() {
		let result = compute_biggest_level(75, 100);
		assert_eq!(result, Some(1));
	}

	#[test]
	fn test_max_on_level_2() {
		let result = compute_biggest_level(32, 128);
		assert_eq!(result, Some(2));
	}

	#[test]
	fn test_max_smaller_than_level_2() {
		let result = compute_biggest_level(31, 128);
		assert_eq!(result, Some(3));
	}
}
