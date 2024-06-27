use std::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use web_sys::{WebGl2RenderingContext, WebGlProgram};

use crate::render_core::shader;

lazy_static! {
	static ref PROGRAM_ID_COUNTER: AtomicUsize = Default::default();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderContext {
	pub context: WebGl2RenderingContext,
	pub program: WebGlProgram,
	program_id: usize,
}

impl ShaderContext {
	fn new(context: &WebGl2RenderingContext, program: &WebGlProgram) -> Self {
		Self {
			context: context.clone(),
			program: program.clone(),
			program_id: PROGRAM_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
		}
	}

	pub fn use_shader(&self) { self.context.use_program(Some(&self.program)); }

	pub fn id(&self) -> usize { self.program_id }
}

pub fn get_planet_shaders(context: &WebGl2RenderingContext) -> Result<ShaderContext, String> {
	let vert_shader = shader::preprocess_and_compile_shader(
		&context,
		WebGl2RenderingContext::VERTEX_SHADER,
		include_str!("shaders/planet.vert"),
	)?;

	let frag_shader = shader::preprocess_and_compile_shader(
		&context,
		WebGl2RenderingContext::FRAGMENT_SHADER,
		include_str!("shaders/planet.frag"),
	)?;

	let program = shader::link_program(&context, &vert_shader, &frag_shader)?;
	Ok(ShaderContext::new(&context, &program))
}

pub fn get_direct_mesh_render_shaders(
	context: &WebGl2RenderingContext,
) -> Result<ShaderContext, String> {
	let vert_shader = shader::preprocess_and_compile_shader(
		&context,
		WebGl2RenderingContext::VERTEX_SHADER,
		include_str!("shaders/direct_mesh.vert"),
	)?;

	let frag_shader = shader::preprocess_and_compile_shader(
		&context,
		WebGl2RenderingContext::FRAGMENT_SHADER,
		include_str!("shaders/passthrough.frag"),
	)?;

	let program = shader::link_program(&context, &vert_shader, &frag_shader)?;
	Ok(ShaderContext::new(&context, &program))
}
