use bevy::{
	core_pipeline::core_3d::Transparent3d,
	ecs as bevy_ecs,
	ecs::system::{lifetimeless::*, SystemParamItem},
	pbr::{MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
	prelude::*,
	render::{
		extract_component::{ExtractComponent, ExtractComponentPlugin},
		mesh::{GpuBufferInfo, MeshVertexBufferLayout},
		primitives::Frustum,
		render_asset::RenderAssets,
		render_phase::{
			AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
			SetItemPipeline, TrackedRenderPass,
		},
		render_resource::*,
		renderer::RenderDevice,
		view::{ExtractedView, Msaa},
		RenderApp, RenderStage,
	},
};
use bevy_ecs::prelude::Component;

use crate::SHADER_HANDLE;
use bevy::render::primitives::Sphere;
use bytemuck::{Pod, Zeroable};

#[derive(Component, Deref)]
pub(super) struct InstanceMaterialData(pub(super) Vec<InstanceData>);

impl ExtractComponent for InstanceMaterialData {
	// type Query = (&'static InstanceMaterialData, &'static Frustum);
    type Query = &'static InstanceMaterialData;
	type Filter = ();

	// Frustum cull at extract
	fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
		//Todo: make frustum planes slightly bigger by half a box width so sphere can be point.
		
        InstanceMaterialData(
			item.0.clone())

        // InstanceMaterialData(
		// 	item.0
		// 		 .0
		// 		.iter()
		// 		.filter_map(|i| {
		// 			item.1
		// 				.intersects_sphere(&Sphere { center: i.position.into(), radius: 5. }, false)
		// 				.then(|| i.clone())
		// 		})
		// 		.collect::<Vec<_>>(),
		// )
	}
}

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugin(ExtractComponentPlugin::<InstanceMaterialData>::default());
		app.sub_app_mut(RenderApp)
			.add_render_command::<Transparent3d, DrawCustom>()
			.init_resource::<CustomPipeline>()
			.init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
			.add_system_to_stage(RenderStage::Queue, queue_custom)
			.add_system_to_stage(RenderStage::Prepare, prepare_instance_buffers);
	}
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub(super) struct InstanceData {
	pub(super) position: Vec3,
	// Destination height of rain.
	pub(super) scale: f32,
	pub(super) color: u32, /* 4], */
	                       /* TODO: color can be a u32!
	                        *  // Unpack the `u32` from the vertex buffer into the `vec4<f32>`
	                        * used by the fragment shader    out.color =
	                        * vec4<f32>((vec4<u32>(vertex.color) >> vec4<u32>(0u, 8u, 16u, 24u))
	                        * & vec4<u32>(255u)) / 255.0; */
}

#[allow(clippy::too_many_arguments)]
fn queue_custom(
	transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
	custom_pipeline: Res<CustomPipeline>,
	msaa: Res<Msaa>,
	mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
	mut pipeline_cache: ResMut<PipelineCache>,
	meshes: Res<RenderAssets<Mesh>>,
	material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<InstanceMaterialData>>,
	mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
	let draw_custom = transparent_3d_draw_functions.read().get_id::<DrawCustom>().unwrap();

	let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

	for (view, mut transparent_phase) in &mut views {
		let rangefinder = view.rangefinder3d();
		for (entity, mesh_uniform, mesh_handle) in &material_meshes {
			if let Some(mesh) = meshes.get(mesh_handle) {
				let key =
					msaa_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
				let pipeline = pipelines
					.specialize(&mut pipeline_cache, &custom_pipeline, key, &mesh.layout)
					.unwrap();
				transparent_phase.add(Transparent3d {
					entity,
					pipeline,
					draw_function: draw_custom,
					distance: rangefinder.distance(&mesh_uniform.transform),
				});
			}
		}
	}
}

#[derive(Component)]
pub struct InstanceBuffer {
	buffer: Buffer,
	length: usize,
}

fn prepare_instance_buffers(
	mut commands: Commands,
	query: Query<(Entity, &InstanceMaterialData)>,
	render_device: Res<RenderDevice>,
) {
	for (entity, instance_data) in &query {
		let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some("instance data buffer"),
			contents: bytemuck::cast_slice(instance_data.as_slice()),
			usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
		});
		commands
			.entity(entity)
			.insert(InstanceBuffer { buffer, length: instance_data.len() });
	}
}

pub struct CustomPipeline {
	shader: Handle<Shader>,
	mesh_pipeline: MeshPipeline,
}

impl FromWorld for CustomPipeline {
	fn from_world(world: &mut World) -> Self {
		CustomPipeline {
			shader: SHADER_HANDLE.typed(),
			mesh_pipeline: world.resource::<MeshPipeline>().clone(),
		}
	}
}

impl SpecializedMeshPipeline for CustomPipeline {
	type Key = MeshPipelineKey;

	fn specialize(
		&self,
		key: Self::Key,
		layout: &MeshVertexBufferLayout,
	) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
		let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
		descriptor.vertex.shader = self.shader.clone();
		descriptor.vertex.buffers.push(VertexBufferLayout {
			array_stride: std::mem::size_of::<InstanceData>() as u64,
			step_mode: VertexStepMode::Instance,
			attributes: vec![
				VertexAttribute {
					format: VertexFormat::Float32x4,
					offset: 0,
					shader_location: 3, /* shader locations 0-2 are taken up by Position, Normal
					                     * and UV attributes */
				},
				VertexAttribute {
					format: VertexFormat::Uint32,
					offset: VertexFormat::Float32x4.size(),
					shader_location: 4,
				},
			],
		});
		descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
		descriptor.layout = Some(vec![
			self.mesh_pipeline.view_layout.clone(),
			self.mesh_pipeline.mesh_layout.clone(),
		]);

		Ok(descriptor)
	}
}

type DrawCustom =
	(SetItemPipeline, SetMeshViewBindGroup<0>, SetMeshBindGroup<1>, DrawMeshInstanced);

pub struct DrawMeshInstanced;

impl EntityRenderCommand for DrawMeshInstanced {
	type Param =
		(SRes<RenderAssets<Mesh>>, SQuery<Read<Handle<Mesh>>>, SQuery<Read<InstanceBuffer>>);
	#[inline]
	fn render<'w>(
		_view: Entity,
		item: Entity,
		(meshes, mesh_query, instance_buffer_query): SystemParamItem<'w, '_, Self::Param>,
		pass: &mut TrackedRenderPass<'w>,
	) -> RenderCommandResult {
		let mesh_handle = mesh_query.get(item).unwrap();
		let instance_buffer = instance_buffer_query.get_inner(item).unwrap();

		let gpu_mesh = match meshes.into_inner().get(mesh_handle) {
			Some(gpu_mesh) => gpu_mesh,
			None => return RenderCommandResult::Failure,
		};

		pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
		pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

		match &gpu_mesh.buffer_info {
			GpuBufferInfo::Indexed { buffer, index_format, count } => {
				pass.set_index_buffer(buffer.slice(..), 0, *index_format);
				pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
			},
			GpuBufferInfo::NonIndexed { vertex_count } => {
				pass.draw(0..*vertex_count, 0..instance_buffer.length as u32);
			},
		}
		RenderCommandResult::Success
	}
}
