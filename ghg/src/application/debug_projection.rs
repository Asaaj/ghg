use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use crate::application::shaders::ShaderContext;
use crate::application::sphere::generate_sphere_with_color;
use crate::application::vertex::{mesh_is_always_visible, BasicMesh};
use crate::interaction_core::user_inputs::LogicalCursorPosition;
use crate::render_core::animation_params::AnimationParams;
use crate::render_core::camera::{Camera, MvpMatrices};
use crate::render_core::frame_sequencer::FrameGate;
use crate::render_core::mesh::{add_mesh, draw_meshes, DrawBuffers, DrawMode, MeshMode};
use crate::render_core::uniform;

fn generate_projection() -> Vec<BasicMesh> {
	let white = nglm::Vec4::repeat(1.0);
	generate_sphere_with_color(2, 6, Some(white))
		.into_iter()
		.map(|mut mesh| {
			mesh.set_visible_fn(mesh_is_always_visible);
			mesh
		})
		.collect()
}

fn generate_drawable_projection(shader_context: ShaderContext) -> Vec<(BasicMesh, DrawBuffers)> {
	let meshes = generate_projection();
	let buffers: Vec<DrawBuffers> =
		meshes.iter().map(|m| add_mesh(&shader_context, m, MeshMode::Static).unwrap()).collect();

	meshes.into_iter().zip(buffers.into_iter()).collect()
}

fn get_cursor_ray(
	cursor_location: LogicalCursorPosition,
	_camera: &Camera,
	mvp: &MvpMatrices,
	screen_width: f32,
	screen_height: f32,
) -> nglm::Vec3 {
	let normalized_device_coords = nglm::vec2(
		(2.0 * cursor_location.x as f32) / screen_width - 1.0,
		-1.0 * ((2.0 * cursor_location.y as f32) / screen_height - 1.0),
	);
	let homogeneous_clip_coords =
		nglm::vec4(normalized_device_coords.x, normalized_device_coords.y, -1.0, 1.0);

	let mut ray_eye = mvp.projection.try_inverse().expect("Failed to invert projection matrix")
		* homogeneous_clip_coords;
	ray_eye = nglm::vec4(ray_eye.x, ray_eye.y, -1.0, 0.0);

	let ray_world = (mvp.view.try_inverse().expect("Failed to invert the view matrix") * ray_eye)
		.xyz()
		.normalize();

	ray_world
}

fn get_sphere_intersection(
	camera: &Camera,
	ray: nglm::Vec3,
	sphere_center: nglm::Vec3,
	sphere_radius: f32,
) -> Option<nglm::Vec3> {
	let ray = ray.normalize();
	let camera_pos = camera.position();

	let difference = camera_pos - sphere_center;
	let a = ray.dot(&ray);
	let b = ray.dot(&difference);
	let c = difference.dot(&difference) - sphere_radius * sphere_radius;
	let delta = b * b - a * c;

	if delta < 0.0 {
		return None;
	}

	let sqrt_delta = delta.sqrt();
	let t_min = (-b - sqrt_delta) / a;
	let t_max = (-b + sqrt_delta) / a;

	if t_max < 0.0 {
		return None;
	}

	let t = if t_min >= 0.0 { t_min } else { t_max };
	let intersection = camera_pos + (t * ray);
	let intersection = nglm::vec3(intersection.x, -intersection.y, intersection.z);
	return Some(intersection);
}

pub async fn draw(
	gate: FrameGate<AnimationParams>,
	shader: ShaderContext,
	camera: Rc<RefCell<Camera>>,
	current_cursor_location: Rc<Cell<Option<LogicalCursorPosition>>>,
) {
	let num_projections = 1;
	let mut projection_locations = vec![nglm::Vec3::zeros()];
	let radius = 0.02;
	let meshes_and_buffers: Vec<_> =
		(0..num_projections).map(|i| generate_drawable_projection(shader.clone())).collect();

	shader.use_shader();

	let mut projection_scale = uniform::new_smart_f32("u_meshScale", &shader);
	let mut projection_location = uniform::new_smart_vec3("u_meshTranslation", &shader);

	let mut model = uniform::new_smart_mat4("u_model", &shader);
	let mut view = uniform::new_smart_mat4("u_view", &shader);
	let mut projection = uniform::new_smart_mat4("u_projection", &shader);

	loop {
		let params = (&gate).await;

		shader.use_shader();

		projection_scale.smart_write(radius);

		let width = params.viewport.width() as i32;
		let height = params.viewport.height() as i32;

		let mvp = camera.deref().borrow().get_perspective_matrices(width, height);

		model.smart_write(mvp.model.clone());
		view.smart_write(mvp.view.clone());
		projection.smart_write(mvp.projection.clone());

		if let Some(cursor_location) = current_cursor_location.get() {
			let cursor_ray = get_cursor_ray(
				cursor_location,
				&camera.deref().borrow(),
				&mvp,
				width as f32,
				height as f32,
			);
			let cursor_intersection = get_sphere_intersection(
				&camera.deref().borrow(),
				cursor_ray,
				nglm::Vec3::zeros(),
				1.0,
			);

			if let Some(intersection) = cursor_intersection {
				projection_locations[0] = intersection;
			} else {
				projection_locations[0] = nglm::Vec3::zeros();
			}
		}

		for (mesh_buffer, location) in
			meshes_and_buffers.iter().zip(projection_locations.iter().cloned())
		{
			projection_location.smart_write(location);
			draw_meshes(
				params.viewport.context(),
				camera.deref().borrow().deref(),
				mesh_buffer,
				DrawMode::Surface,
			);
		}
	}
}
