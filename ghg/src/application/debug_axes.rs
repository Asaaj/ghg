use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use crate::application::shaders::ShaderContext;
use crate::application::vertex::{BasicMesh, Vertex};
use crate::interaction_core::user_inputs::LogicalCursorPosition;
use crate::render_core::animation_params::AnimationParams;
use crate::render_core::camera::Camera;
use crate::render_core::frame_sequencer::FrameGate;
use crate::render_core::mesh::{add_mesh, draw_meshes, DrawBuffers, DrawMode, MeshMode};
use crate::render_core::uniform;

fn generate_axes(length: f32, thickness: f32) -> Vec<BasicMesh> {
	let len = length;
	let wdt = thickness;

	let red = nglm::vec4(1.0, 0.0, 0.0, 1.0);
	let green = nglm::vec4(0.0, 1.0, 0.0, 1.0);
	let blue = nglm::vec4(0.0, 0.0, 1.0, 1.0);

	let x_vertices = vec![
		Vertex::from_vecs(nglm::vec3(0.0, 0.0, 0.0), nglm::Vec3::y(), red),
		Vertex::from_vecs(nglm::vec3(len, 0.0, 0.0), nglm::Vec3::y(), red),
		Vertex::from_vecs(nglm::vec3(len, 0.0, wdt), nglm::Vec3::y(), red),
		Vertex::from_vecs(nglm::vec3(0.0, 0.0, wdt), nglm::Vec3::y(), red),
	];

	let y_vertices = vec![
		Vertex::from_vecs(nglm::vec3(0.0, 0.0, 0.0), nglm::Vec3::z(), green),
		Vertex::from_vecs(nglm::vec3(0.0, -len, 0.0), nglm::Vec3::z(), green),
		Vertex::from_vecs(nglm::vec3(wdt, -len, 0.0), nglm::Vec3::z(), green),
		Vertex::from_vecs(nglm::vec3(wdt, 0.0, 0.0), nglm::Vec3::z(), green),
	];

	let z_vertices = vec![
		Vertex::from_vecs(nglm::vec3(0.0, 0.0, 0.0), nglm::Vec3::y(), blue),
		Vertex::from_vecs(nglm::vec3(0.0, 0.0, len), nglm::Vec3::y(), blue),
		Vertex::from_vecs(nglm::vec3(0.0, -wdt, len), nglm::Vec3::y(), blue),
		Vertex::from_vecs(nglm::vec3(0.0, -wdt, 0.0), nglm::Vec3::x(), blue),
	];

	let indices = vec![0, 1, 2, 0, 2, 3];

	let x_axis = BasicMesh::with_contents(x_vertices, indices.clone());
	let y_axis = BasicMesh::with_contents(y_vertices, indices.clone());
	let z_axis = BasicMesh::with_contents(z_vertices, indices.clone());

	vec![x_axis, y_axis, z_axis]
}

fn generate_drawable_axes(
	shader_context: ShaderContext,
	length: f32,
	thickness: f32,
) -> Vec<(BasicMesh, DrawBuffers)> {
	let meshes = generate_axes(length, thickness);
	let buffers: Vec<DrawBuffers> =
		meshes.iter().map(|m| add_mesh(&shader_context, m, MeshMode::Static).unwrap()).collect();

	meshes.into_iter().zip(buffers.into_iter()).collect()
}

pub async fn draw(
	gate: FrameGate<AnimationParams>,
	shader: ShaderContext,
	camera: Rc<RefCell<Camera>>,
	current_cursor_location: Rc<Cell<Option<LogicalCursorPosition>>>,
) {
	let meshes_and_buffers = generate_drawable_axes(shader.clone(), 0.2, 0.02);

	shader.use_shader();

	let mut scale = uniform::new_smart_f32("u_meshScale", &shader);
	let mut location = uniform::new_smart_vec3("u_meshTranslation", &shader);

	let mut model = uniform::new_smart_mat4("u_model", &shader);
	let mut view = uniform::new_smart_mat4("u_view", &shader);
	let mut projection = uniform::new_smart_mat4("u_projection", &shader);

	loop {
		let params = (&gate).await;

		shader.use_shader();

		scale.smart_write(1.0);
		location.smart_write(nglm::Vec3::zeros());

		let width = params.viewport.width() as i32;
		let height = params.viewport.height() as i32;

		let mvp = camera.deref().borrow().get_perspective_matrices(width, height);

		model.smart_write(mvp.model.clone());
		view.smart_write(mvp.view.clone());
		projection.smart_write(mvp.projection.clone());

		draw_meshes(
			params.viewport.context(),
			camera.deref().borrow().deref(),
			&meshes_and_buffers,
			DrawMode::Surface,
		);
	}
}
