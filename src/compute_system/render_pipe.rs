use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId, PipelineCache,
            ShaderStages, StorageTextureAccess, TextureFormat, binding_types,
        },
        renderer::RenderDevice,
    },
};


use super::{shared,EnviromentData,PushConstants, comp_utils};

#[derive(Resource)]
pub struct ComputeRenderPipeline {
    pub bind_group_layouts: Vec<BindGroupLayout>,
    pub main_pipeline: CachedComputePipelineId,
}

impl FromWorld for ComputeRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let shader = world.load_asset(shared::COMPUTE_SHADER_PATH);

        let out_layout = render_device.create_bind_group_layout(
            "ComputeRenderImages",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    binding_types::texture_storage_2d(
                        TextureFormat::Rgba8Unorm,
                        StorageTextureAccess::ReadOnly,
                    ),
                    binding_types::texture_storage_2d(
                        TextureFormat::Rgba8Unorm,
                        StorageTextureAccess::WriteOnly,
                    ),
                ),
            ),
        );

        let shared_layout = render_device.create_bind_group_layout(
            "ComputeShared",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    binding_types::texture_depth_2d(),
                    binding_types::uniform_buffer::<EnviromentData>(false),
                    binding_types::storage_buffer_read_only::<i32>(false),
                    binding_types::uniform_buffer::<PushConstants>(false)
                ),
            ),
        );

        let bind_group_layouts = vec![out_layout, shared_layout];

        let pipeline_cache = world.resource::<PipelineCache>();
        let main_pipeline = pipeline_cache.queue_compute_pipeline(
            comp_utils::compute_pipeline_descriptor(shader, "main", &bind_group_layouts),
        );

        ComputeRenderPipeline {
            bind_group_layouts,
            main_pipeline,
        }
    }
}
