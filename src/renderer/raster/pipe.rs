
use bevy::{
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{
        MeshInputUniform, MeshPipeline, MeshPipelineKey, MeshPipelineViewLayoutKey,
        MeshUniform, RenderMeshInstances
    },
    prelude::*,
    render::{
        batching::{
            GetBatchData, GetFullBatchData,
            gpu_preprocessing::{
                IndirectParametersCpuMetadata, UntypedPhaseIndirectParametersBuffers,
            },
        },
        mesh::{RenderMesh, allocator::MeshAllocator},
        render_asset::RenderAssets,
        render_resource::{
            BindGroupLayoutDescriptor, CompareFunction, DepthStencilState, TextureFormat,
            ColorTargetState, ColorWrites, Face, FragmentState, PrimitiveState,
            RenderPipelineDescriptor, SpecializedMeshPipeline, SpecializedMeshPipelineError,
            VertexState,
        },
        sync_world::MainEntity,
    },
};
use nonmax::NonMaxU32;


// This contains the state needed to specialize a mesh pipeline
#[derive(Resource)]
pub struct GBufferMeshPipeline {
    /// The base mesh pipeline defined by bevy
    ///
    /// This isn't required, but if you want to use a bevy `Mesh` it's easier when you
    /// have access to the base `MeshPipeline` that bevy already defines
    mesh_pipeline: MeshPipeline,
    /// Stores the shader used for this pipeline directly on the pipeline.
    /// This isn't required, it's only done like this for simplicity.
    shader_handle: Handle<Shader>,
    material_layout_desc: BindGroupLayoutDescriptor,
}
impl GBufferMeshPipeline {
    pub fn new(
        mesh_pipeline: MeshPipeline,
        shader_handle: Handle<Shader>,
        mat_desc: BindGroupLayoutDescriptor,
    ) -> Self {
        Self {
            mesh_pipeline,
            shader_handle,
            material_layout_desc: mat_desc,
        }
    }
}
impl SpecializedMeshPipeline for GBufferMeshPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        // We will only use the position of the mesh in our shader so we only need to specify that
        let mut vertex_attributes = Vec::new();
        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            // Make sure this matches the shader location
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }
        if layout.0.contains(Mesh::ATTRIBUTE_UV_0) {
              vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(1));
        }

        // This will automatically generate the correct `VertexBufferLayout` based on the vertex attributes
        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        let targets = vec![
            Some(ColorTargetState {
                format: TextureFormat::Rgba8Unorm, // Albedo
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
            Some(ColorTargetState {
                format: TextureFormat::Rgba16Float, // Normal
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
             Some(ColorTargetState {
                format: TextureFormat::Rgba8Unorm, // Material
                blend: None,
                write_mask: ColorWrites::ALL,
            }),
        ];

        let view_layout = self
            .mesh_pipeline
            .get_view_layout(MeshPipelineViewLayoutKey::from(key));
        Ok(RenderPipelineDescriptor {
            label: Some("Specialized Mesh Pipeline".into()),
            // We want to reuse the data from bevy so we use the same bind groups as the default
            // mesh pipeline
            layout: vec![
                // Bind group 0 is the view uniform
                view_layout.main_layout.clone(),
                // Bind group 1 is empty
                view_layout.empty_layout.clone(),
                // Bind group 2 is the mesh uniform
                self.mesh_pipeline.mesh_layouts.model_only.clone(),
                self.material_layout_desc.clone(),
            ],
            vertex: VertexState {
                shader: self.shader_handle.clone(),
                buffers: vec![vertex_buffer_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader_handle.clone(),
                 targets,
                ..default()
            }),
            primitive: PrimitiveState {
                topology: key.primitive_topology(),
                cull_mode: Some(Face::Back),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: default(),
                bias: default(),
            }),
            // It's generally recommended to specialize your pipeline for MSAA,
            // but it's not always possible
            ..default()
        })
    }
}

impl GetBatchData for GBufferMeshPipeline {
    type Param = (
        SRes<RenderMeshInstances>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
    );
    type CompareData = AssetId<Mesh>;
    type BufferData = MeshUniform;

    fn get_batch_data(
        (mesh_instances, _render_assets, mesh_allocator): &SystemParamItem<Self::Param>,
        (_entity, main_entity): (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_batch_data` should never be called in GPU mesh uniform \
                building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };
        let mesh_uniform = {
            let mesh_transforms = &mesh_instance.transforms;
            let (local_from_world_transpose_a, local_from_world_transpose_b) =
                mesh_transforms.world_from_local.inverse_transpose_3x3();
            MeshUniform {
                world_from_local: mesh_transforms.world_from_local.to_transpose(),
                previous_world_from_local: mesh_transforms.previous_world_from_local.to_transpose(),
                lightmap_uv_rect: UVec2::ZERO,
                local_from_world_transpose_a,
                local_from_world_transpose_b,
                flags: mesh_transforms.flags,
                first_vertex_index,
                current_skin_index: u32::MAX,
                material_and_lightmap_bind_group_slot: 0,
                tag: 0,
                pad: 0,
            }
        };
        Some((mesh_uniform, None))
    }
}

impl GetFullBatchData for GBufferMeshPipeline {
    type BufferInputData = MeshInputUniform;

    fn get_index_and_compare_data(
        (mesh_instances, _, _): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<(NonMaxU32, Option<Self::CompareData>)> {
        // This should only be called during GPU building.
        let RenderMeshInstances::GpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_index_and_compare_data` should never be called in CPU mesh uniform building \
                mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        Some((
            mesh_instance.current_uniform_index,
            mesh_instance
                .should_batch()
                .then_some(mesh_instance.mesh_asset_id),
        ))
    }

    fn get_binned_batch_data(
        (mesh_instances, _render_assets, mesh_allocator): &SystemParamItem<Self::Param>,
        main_entity: MainEntity,
    ) -> Option<Self::BufferData> {
        let RenderMeshInstances::CpuBuilding(ref mesh_instances) = **mesh_instances else {
            error!(
                "`get_binned_batch_data` should never be called in GPU mesh uniform building mode"
            );
            return None;
        };
        let mesh_instance = mesh_instances.get(&main_entity)?;
        let first_vertex_index =
            match mesh_allocator.mesh_vertex_slice(&mesh_instance.mesh_asset_id) {
                Some(mesh_vertex_slice) => mesh_vertex_slice.range.start,
                None => 0,
            };

        Some(MeshUniform::new(
            &mesh_instance.transforms,
            first_vertex_index,
            mesh_instance.material_bindings_index.slot,
            None,
            None,
            None,
        ))
    }

    fn write_batch_indirect_parameters_metadata(
        indexed: bool,
        base_output_index: u32,
        batch_set_index: Option<NonMaxU32>,
        indirect_parameters_buffers: &mut UntypedPhaseIndirectParametersBuffers,
        indirect_parameters_offset: u32,
    ) {
        // Note that `IndirectParameters` covers both of these structures, even
        // though they actually have distinct layouts. See the comment above that
        // type for more information.
        let indirect_parameters = IndirectParametersCpuMetadata {
            base_output_index,
            batch_set_index: match batch_set_index {
                None => !0,
                Some(batch_set_index) => u32::from(batch_set_index),
            },
        };

        if indexed {
            indirect_parameters_buffers
                .indexed
                .set(indirect_parameters_offset, indirect_parameters);
        } else {
            indirect_parameters_buffers
                .non_indexed
                .set(indirect_parameters_offset, indirect_parameters);
        }
    }

    fn get_binned_index(
        _param: &SystemParamItem<Self::Param>,
        _query_item: MainEntity,
    ) -> Option<NonMaxU32> {
        None
    }
}
