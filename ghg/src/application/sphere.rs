use nglm::{Vec2, Vec3, Vec4};

use crate::application::vertex::{BasicMesh, Vertex};
use crate::render_core::camera::Camera;
#[allow(unused_imports)]
use crate::utils::prelude::*;

fn is_sphere_section_visible(sphere: &BasicMesh, camera: &Camera) -> bool {
	let mesh_center = sphere.center;
	if mesh_center.is_none() {
		ghg_error!("Invalid sphere!");
		return false;
	}
	let camera_position = camera.position();
	let dist = camera_position.metric_distance(&mesh_center.unwrap());
	let camera_from_origin = camera_position.magnitude();

	dist <= camera_from_origin
}

// Thanks, Sebastian Lague. https://www.youtube.com/watch?v=sLqXFF8mlEU
pub fn generate_sphere(subdivisions: u32, points_per_subdivision: u32) -> Vec<BasicMesh> {
	generate_sphere_with_color(subdivisions, points_per_subdivision, None)
}

pub fn generate_sphere_with_color(
	subdivisions: u32,
	points_per_subdivision: u32,
	face_color: Option<Vec4>,
) -> Vec<BasicMesh> {
	let subface_generator = QuadSubface {};
	cube_normals()
		.iter()
		.map(|n| {
			generate_face::<QuadSubface>(
				subface_generator.clone(),
				n,
				subdivisions,
				points_per_subdivision,
				face_color.clone(),
			)
		})
		.flatten()
		.collect()
}

fn cube_normals() -> Vec<Vec3> {
	vec![
		Vec3::ith(1, 1.0),  // up
		Vec3::ith(1, -1.0), // down
		Vec3::ith(0, 1.0),  // left
		Vec3::ith(0, -1.0), // right
		Vec3::ith(2, 1.0),  // front
		Vec3::ith(2, -1.0), // back
	]
}

fn generate_face<T: SubfaceGenerator + Clone>(
	subface_generator: T,
	normal: &Vec3,
	subdivisions: u32,
	points_per_subdivision: u32,
	face_color: Option<Vec4>,
) -> Vec<BasicMesh> {
	let mut meshes = Vec::new();

	let subdivision_size = 1.0 / (subdivisions as f32);

	for y in 0..subdivisions {
		for x in 0..subdivisions {
			let subdivision_x = (x as f32) * subdivision_size;
			let subdivision_y = (y as f32) * subdivision_size;
			let subdivision_start = nglm::vec2(subdivision_x, subdivision_y);

			meshes.push(generate_subface(
				subface_generator.clone(),
				normal,
				points_per_subdivision,
				subdivision_start,
				subdivision_size,
				face_color.clone(),
			));
		}
	}

	meshes
}

fn generate_subface<T: SubfaceGenerator>(
	subface_generator: T,
	face_normal: &Vec3,
	points_per_side: u32,
	subdivision_start: Vec2,
	subdivision_side_length: f32,
	face_color: Option<Vec4>,
) -> BasicMesh {
	let face_color = face_color.unwrap_or(determine_face_color(face_normal));

	let axis_a = nglm::vec3(face_normal.y, face_normal.z, face_normal.x);
	let axis_b = nglm::cross(face_normal, &axis_a);

	let (num_vertices, num_indices) = subface_generator.vertex_and_index_size(points_per_side);
	let mut mesh = BasicMesh::with_capacities(num_vertices, num_indices);
	mesh.set_visible_fn(is_sphere_section_visible);

	for y in 0..points_per_side {
		for x in 0..points_per_side {
			let t: Vec2 = nglm::vec2(
				(x as f32) / ((points_per_side as f32) - 1.0) * subdivision_side_length
					+ subdivision_start.x,
				(y as f32) / ((points_per_side as f32) - 1.0) * subdivision_side_length
					+ subdivision_start.y,
			);

			let point: Vec3 =
				(face_normal + axis_a * (2.0 * t.x - 1.0) + axis_b * (2.0 * t.y - 1.0)).into();

			let sphere_point = point_on_cube_to_point_on_sphere(point);
			let normal: Vec3 = sphere_point.normalize();

			subface_generator.add_for_point(
				&mut mesh,
				x,
				y,
				face_normal,
				&normal,
				&face_color,
				points_per_side,
				&sphere_point,
			);
		}
	}

	mesh
}

fn point_on_cube_to_point_on_sphere(p: Vec3) -> Vec3 {
	let x2 = p.x * p.x;
	let y2 = p.y * p.y;
	let z2 = p.z * p.z;
	let x = p.x * (1.0 - (y2 + z2) / 2.0 + (y2 * z2) / 3.0).sqrt();
	let y = p.y * (1.0 - (z2 + x2) / 2.0 + (z2 * x2) / 3.0).sqrt();
	let z = p.z * (1.0 - (x2 + y2) / 2.0 + (x2 * y2) / 3.0).sqrt();

	nglm::vec3(x, y, z)
}

fn determine_face_color(_normal: &Vec3) -> Vec4 {
	let r = js_sys::Math::random() as f32;
	let g = js_sys::Math::random() as f32;
	let b = js_sys::Math::random() as f32;
	nglm::vec4(r, g, b, 1.0)
}

trait SubfaceGenerator {
	fn add_for_point(
		&self,
		mesh: &mut BasicMesh,
		x: u32,
		y: u32,
		face_normal: &Vec3,
		normal: &Vec3,
		face_color: &Vec4,
		points_per_side: u32,
		sphere_point: &Vec3,
	);

	fn vertex_and_index_size(&self, points_per_side: u32) -> (usize, usize);
}

#[derive(Clone)]
struct QuadSubface();

impl SubfaceGenerator for QuadSubface {
	fn add_for_point(
		&self,
		mesh: &mut BasicMesh,
		x: u32,
		y: u32,
		_face_normal: &Vec3,
		normal: &Vec3,
		face_color: &Vec4,
		points_per_side: u32,
		sphere_point: &Vec3,
	) {
		mesh.push_vertex(Vertex::from_vecs(
			sphere_point.clone(),
			normal.clone(),
			face_color.clone(),
		));

		if x != points_per_side - 1 && y != points_per_side - 1 {
			let vertex_index = y * points_per_side + x;
			mesh.push_index(vertex_index);
			mesh.push_index(vertex_index + points_per_side + 1);
			mesh.push_index(vertex_index + points_per_side);
			mesh.push_index(vertex_index);
			mesh.push_index(vertex_index + 1);
			mesh.push_index(vertex_index + points_per_side + 1);
		}
	}

	fn vertex_and_index_size(&self, points_per_side: u32) -> (usize, usize) {
		(
			(points_per_side * points_per_side) as usize,
			((points_per_side - 1) * (points_per_side - 1) * 6) as usize,
		)
	}
}
