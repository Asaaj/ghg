use std::path::Path;

use image::LumaA;
use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;

use crate::application::image_utility::biggest_mipmap_level;
use crate::application::shaders::ShaderContext;
use crate::render_core::animation_params::AnimationParams;
use crate::render_core::frame_sequencer::FrameGate;
use crate::render_core::image::load_into_texture_with_filters;
use crate::render_core::texture_provider::TextureProvider;
use crate::render_core::uniform;
use crate::request_data::fetch_bytes;
use crate::utils::prelude::*;

const COUNTRY_IMAGE_MAX_SIZE: usize = 21_600;

async fn load_country_data(
	shader_context: ShaderContext,
	texture_index: u32,
) -> Result<(), JsValue> {
	let mipmap_level =
		biggest_mipmap_level(shader_context.context.clone(), COUNTRY_IMAGE_MAX_SIZE)?;
	let country_root = Path::new("images/countries");
	let country_map_image = country_root.join(format!("{mipmap_level}/full.png").as_str());

	let texture = fetch_bytes(country_map_image.to_str().unwrap()).await?;
	shader_context.use_shader();
	load_into_texture_with_filters::<LumaA<u8>>(
		shader_context.context.clone(),
		&texture,
		WebGl2RenderingContext::TEXTURE0 + texture_index,
		WebGl2RenderingContext::NEAREST, // Avoids weird boundary aliasing
		WebGl2RenderingContext::NEAREST,
	)?;

	Ok(())
}

pub async fn draw_borders(
	gate: FrameGate<AnimationParams>,
	shader_context: ShaderContext,
	mut texture_provider: TextureProvider,
) {
	let texture_index = texture_provider.take();

	shader_context.use_shader();

	let _texture_uniform =
		uniform::init_smart_i32("s_countryMap", &shader_context, texture_index as i32);
	let load_result = load_country_data(shader_context.clone(), texture_index).await;
	if !load_result.is_ok() {
		ghg_error!("Failed to load country data: {:?}", load_result);
		return;
	}

	loop {
		let _params = (&gate).await;
	}
}
