use super::utils;
use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice, texture::CachedTexture},
};

#[derive(Resource, Default)]

pub struct GBufferTargets {
    pub albedo: Option<CachedTexture>,
    pub normal: Option<CachedTexture>,
    pub ray_depth: Option<CachedTexture>,
    pub raster_depth: Option<CachedTexture>,
    pub final_out: Option<CachedTexture>,
}

#[derive(Resource, Default)]
pub struct RenderBindGroups {
    pub raymarch: Option<BindGroup>,
    pub raster: Option<BindGroup>,
    pub lighting: Option<BindGroup>,
    pub shared: Option<BindGroup>,
}

#[derive(Resource)]
pub struct GBufferLayout {
    pub read_layout: BindGroupLayout,
    pub write_layout: BindGroupLayout,
    pub raster_layout: BindGroupLayout,
}

impl FromWorld for GBufferLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let read_layout = render_device.create_bind_group_layout(
            "ReadGBuffer",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                        binding_types::texture_storage_2d(
                        TextureFormat::Rgba8Unorm,
                        StorageTextureAccess::ReadOnly,
                    ),
                    binding_types::texture_storage_2d(
                        TextureFormat::Rgba16Float,
                        StorageTextureAccess::ReadOnly,
                    ),
                    binding_types::texture_storage_2d(
                        TextureFormat::R32Float,
                        StorageTextureAccess::ReadOnly,
                    ),
                     binding_types::texture_storage_2d(
                        TextureFormat::Rgba8Unorm,
                        StorageTextureAccess::WriteOnly,
                    ), // Final
                ),
            ),
        );
        let write_layout = render_device.create_bind_group_layout(
            "WriteGBuffer",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    binding_types::texture_storage_2d(
                        TextureFormat::Rgba8Unorm,
                        StorageTextureAccess::WriteOnly,
                    ),
                    binding_types::texture_storage_2d(
                        TextureFormat::Rgba16Float,
                        StorageTextureAccess::WriteOnly,
                    ),
                    binding_types::texture_storage_2d(
                        TextureFormat::R32Float,
                        StorageTextureAccess::WriteOnly,
                    ),
                ),
            ),
        );
         let raster_layout = render_device.create_bind_group_layout(
            "RasterGBuffer",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    binding_types::texture_2d(
                        TextureSampleType::Float { filterable: true }
                    ),
                    binding_types::sampler(SamplerBindingType::Filtering),
                    binding_types::texture_storage_2d(
                        TextureFormat::R32Float,
                        StorageTextureAccess::ReadOnly,
                    ),
                ),
            ),
        );

        Self {
            read_layout,
            write_layout,
            raster_layout,
        }
    }
}

#[derive(Resource)]
pub struct SharedLayouts {
    pub layout: BindGroupLayout,
}

impl FromWorld for SharedLayouts {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            "SharedLayout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    binding_types::texture_depth_2d(),
                    binding_types::uniform_buffer::<utils::EnviromentData>(false),
                    binding_types::storage_buffer_read_only::<i32>(false),
                    binding_types::uniform_buffer::<utils::PushConstants>(false),
                ),
            ),
        );
        Self { layout }
    }
}
