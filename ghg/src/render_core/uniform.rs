use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::sync::Mutex;

use lazy_static::lazy_static;
use paste::paste;
use web_sys::{WebGl2RenderingContext, WebGlUniformLocation};

use crate::application::shaders::ShaderContext;
#[allow(unused_imports)]
use crate::utils::prelude::*;

#[derive(Clone, Debug, Default)]
struct UniformValueTracker<T: UniformValue> {
	uniforms: HashMap<(usize, String), T>,
}

impl<T: Clone + Debug + PartialEq + UniformValue> UniformValueTracker<T> {
	fn update(&mut self, uniform: &Uniform<T>, value: T) -> bool {
		let key = (uniform.shader_context.id(), uniform.name.clone());

		if let Some(saved) = self.uniforms.get_mut(&key) {
			if *saved != value {
				*saved = value;
				return true;
			}
		} else {
			self.uniforms.insert(key, value);
			return true;
		}

		false
	}
}

trait GetUniformValueTracker {
	fn get_uniform_tracker() -> &'static Mutex<UniformValueTracker<Self>>
	where
		Self: UniformValue + Sized;
}

/// This provides a simple wrapper type for writing to uniform values.
/// Uniform<T> provides strongly-typed uniform values and a simple interface
/// for writing the value to the GPU. SmartUniform<T> is an additional layer of
/// "safety", which allows for writing parameters only when they have changed.
/// This helps simplify the process of updating parameters during an animation
/// loop, because you can simply call smart_write every time, and nothing will
/// happen if the parameter hasn't changed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)] // name isn't used, but it is useful. TODO: Remove it if not debug
pub struct Uniform<T: Debug> {
	name: String,
	shader_context: ShaderContext,
	location: Option<WebGlUniformLocation>,
	phantom_value: PhantomData<T>, // Strongly-typed Uniforms are important
}

impl<T: Clone + Debug + PartialEq + UniformValue> Uniform<T> {
	pub fn new(name: &str, shader_context: &ShaderContext) -> Self {
		let location = shader_context.context.get_uniform_location(&shader_context.program, name);
		Self {
			name: name.to_owned(),
			shader_context: shader_context.clone(),
			location,
			phantom_value: PhantomData,
		}
	}

	pub fn write_unchecked(&self, t: T) {
		t.write_to_program(&self.shader_context.context, &self.location);
	}
}

#[derive(Debug)]
pub struct SmartUniform<T: Debug + UniformValue + 'static> {
	uniform: Uniform<T>,
	uniform_tracker: &'static Mutex<UniformValueTracker<T>>,
}

impl<T: Clone + Debug + PartialEq + UniformValue + GetUniformValueTracker + 'static>
	SmartUniform<T>
{
	pub fn new(name: &str, shader_context: &ShaderContext) -> Self {
		Self {
			uniform: Uniform::new(name, shader_context),
			uniform_tracker: T::get_uniform_tracker(),
		}
	}

	pub fn smart_write(&mut self, t: T) {
		match self.uniform_tracker.lock() {
			Ok(mut tracker) => {
				if tracker.deref_mut().update(&self.uniform, t.clone()) {
					self.uniform.write_unchecked(t);
				}
			}
			Err(_) => {
				ghg_error!("Failed to lock uniform tracker: {:?}", self.uniform);
			}
		}
	}
}

pub trait UniformValue {
	fn write_to_program(
		self,
		context: &WebGl2RenderingContext,
		location: &Option<WebGlUniformLocation>,
	) where
		Self: Sized;
}

macro_rules! impl_uniform_creator_fns {
    ($type_name:ty, $short_name:ident) => {
        paste! {
            #[allow(dead_code)] // Used in doc string, but the compiler still complains
            const [< $short_name:upper _STR >]: &str = stringify!($type_name); // TODO: This means this macro has to come first.

            #[allow(dead_code)]
            #[doc = "Creates a new `Uniform<" [< $short_name:upper _STR >] ">`."]
            /// Creates a new Uniform<> of the given type.
            pub fn [< new_ $short_name >](name: &str, shader_context: &ShaderContext) -> Uniform<$type_name> {
                Uniform::new(name, shader_context)
            }

            #[allow(dead_code)]
            #[doc = "Creates and initializes a new `Uniform<" [< $short_name:upper _STR >] ">`."]
            /// Creates and initializes a new Uniform<> of the given type.
            pub fn [< init_ $short_name >](name: &str, shader_context: &ShaderContext, value: $type_name) -> Uniform<$type_name> {
                let u = Uniform::new(name, shader_context);
                u.write_unchecked(value);
                u
            }
        }
    };
}

macro_rules! impl_smart_uniform_creator_fns {
    ($type_name:ty, $short_name:ident, $tracker_name:ident) => {
        paste! {
            #[allow(dead_code)]
            #[doc = "Creates a new `SmartUniform<" [< $short_name:upper _STR >] ">`."]
            /// Creates a new SmartUniform<> of the given type.
            pub fn [< new_smart_ $short_name >](name: &str, shader_context: &ShaderContext) -> SmartUniform<$type_name> {
                SmartUniform::new(name, shader_context)
            }

            #[allow(dead_code)]
            #[doc = "Creates and initializes a new `SmartUniform<" [< $short_name:upper _STR >] ">`."]
            /// Creates and initializes a new SmartUniform<> of the given type.
            pub fn [< init_smart_ $short_name >](name: &str, shader_context: &ShaderContext, value: $type_name) -> SmartUniform<$type_name> {
                let mut u = SmartUniform::new(name, shader_context);
                u.smart_write(value);
                u
            }

        }

        lazy_static! {
            static ref $tracker_name: Mutex<UniformValueTracker<$type_name>> = Default::default();
        }

        impl GetUniformValueTracker for $type_name {
            fn get_uniform_tracker() -> &'static Mutex<UniformValueTracker<Self>> {
                &$tracker_name
            }
        }
    };
}

macro_rules! impl_uniform {
    // Self is a primitive type; pass self directly to the OpenGL function call
    ($type_name:ty, $short_name:ident, $gl_call:ident) => {
        impl UniformValue for $type_name {
            fn write_to_program(self, context: &WebGl2RenderingContext, location: &Option<WebGlUniformLocation>) {
                context.$gl_call(location.as_ref(), self);
            }
        }

        impl_uniform_creator_fns!($type_name, $short_name);
        paste! {
            impl_smart_uniform_creator_fns!($type_name, $short_name, [< UNIFORM_VALUE_ $short_name >]);
        }
    };

    // Self is a primitive type, and its own short name; pass self directly to the OpenGL function call
    ($type_name:ident, $gl_call:ident) => {
        impl_uniform!($type_name, $type_name, $gl_call);
    };


    // Ugly form that takes parameters. You can pass these things:
    //    - self.some_field
    //    - call self.some_method()
    //    - just some_expression
    // Definitely incomplete and hacky, but I'm learning macros.
    (
        $type_name:ty, $short_name:ident, $gl_call:ident,
        $( $(self.$field:ident)? $(call self.$method:ident())? $(just $param:expr)? ),+
    ) => {
        impl UniformValue for $type_name {
            fn write_to_program(self, context: &WebGl2RenderingContext, location: &Option<WebGlUniformLocation>) {
                context.$gl_call(location.as_ref(), $( $(self.$field)* $(self.$method())* $($param)* ,)+);
            }
        }

        impl_uniform_creator_fns!($type_name, $short_name);
        paste! {
            impl_smart_uniform_creator_fns!($type_name, $short_name, [< UNIFORM_VALUE_ $short_name >]);
        }
    };
}

impl_uniform!(i32, uniform1i);
impl_uniform!(f32, uniform1f);
impl_uniform!(nglm::Vec3, vec3, uniform3f, self.x, self.y, self.z);
impl_uniform!(nglm::Vec4, vec4, uniform4f, self.x, self.y, self.z, self.w);
impl_uniform!(nglm::Mat4, mat4, uniform_matrix4fv_with_f32_array, just false, call self.as_slice());

// TODO: Uh... This switches column/row. Is that expected?
impl_uniform!(nglm::Mat4x3, mat4x3, uniform_matrix3x4fv_with_f32_array, just false, call self.as_slice());

// TODO: Way more implementations
