use bevy::{
    prelude::*,
    render::{
        render_graph::RenderLabel, render_resource::{AsBindGroup, BindGroup, BindGroupLayoutDescriptor, Extent3d, ShaderType, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView}, texture::CachedTexture
    },
    shader::ShaderRef,
};
use bytemuck::{Pod, Zeroable};

pub const RASTER_SHADER_PATH: &str = "shaders/raster.wgsl";
pub const RAYMARCH_SHADER_PATH: &str = "shaders/raymarch.wgsl";
pub const LIGHTING_SHADER_PATH: &str = "shaders/lighting.wgsl";
pub const POST_SHADER_PATH: &str = "shaders/post.wgsl";

#[derive(Default)]

pub struct GBufferTargets {
    albedo: Option<CachedTexture>,
    normal: Option<CachedTexture>,
    material: Option<CachedTexture>,
    raster_depth: Option<CachedTexture>,
    combined_depth: Option<CachedTexture>,
}

impl GBufferTargets {
    pub fn albedo_view(&self) -> Option<&TextureView> {
        let Some(albedo) = &self.albedo else {
            return None;
        };
        Some(&albedo.default_view)
    }
    pub fn normal_view(&self) -> Option<&TextureView> { 
        let Some(normal) = &self.normal else {
            return None;
        };
        Some(&normal.default_view)
    }
    pub fn material_view(&self) -> Option<&TextureView> {
        let Some(material) = &self.material else {
            return None;
        };
        Some(&material.default_view)
    }
    pub fn raster_depth_view(&self) -> Option<&TextureView> {
        let Some(raster_depth) = &self.raster_depth else {
            return None;
        };
        Some(&raster_depth.default_view)
    }
    pub fn combined_depth_view(&self) -> Option<&TextureView> {
        let Some(combined_depth) = &self.combined_depth else {
            return None;
        };
        Some(&combined_depth.default_view)
    }
    pub fn set_albedo(&mut self, albedo: CachedTexture) {
        self.albedo = Some(albedo);
    }
    pub fn set_normal(&mut self, normal: CachedTexture) {
        self.normal = Some(normal);
    }
    pub fn set_material(&mut self, material: CachedTexture) {
        self.material = Some(material);
    }
    pub fn set_raster_depth(&mut self, raster_depth: CachedTexture) {
        self.raster_depth = Some(raster_depth);
    }
    pub fn set_combined_depth(&mut self, combined_depth: CachedTexture) {
        self.combined_depth = Some(combined_depth);
    }

}

#[derive(Resource,Default)]
pub struct SceneTargets {
   gbuffer: GBufferTargets,
   hdr: Option<CachedTexture>,
   ldr: Option<CachedTexture>
}

impl SceneTargets {
    pub fn hdr_view(&self) -> Option<&TextureView> {
        let Some(hdr) = &self.hdr else {
            return None;
        };
        Some(&hdr.default_view)
    }
    pub fn ldr_view(&self) -> Option<&TextureView> {
        let Some(ldr) = &self.ldr else {
            return None;
        };
        Some(&ldr.default_view)
    }
    pub fn set_hdr(&mut self, hdr: CachedTexture) {
        self.hdr = Some(hdr);
    }
    pub fn set_ldr(&mut self, ldr: CachedTexture) {
        self.ldr = Some(ldr);
    }
    pub fn set_albedo(&mut self, albedo: CachedTexture) {
        self.gbuffer.set_albedo(albedo);
    }
    pub fn set_normal(&mut self, normal: CachedTexture) {
        self.gbuffer.set_normal(normal);
    }
    pub fn set_material(&mut self, material: CachedTexture) {
        self.gbuffer.set_material(material);
    }
    pub fn set_raster_depth(&mut self, raster_depth: CachedTexture) {
        self.gbuffer.set_raster_depth(raster_depth);
    }
    pub fn set_combined_depth(&mut self, combined_depth: CachedTexture) {
        self.gbuffer.set_combined_depth(combined_depth);
    }
    pub fn albedo_view(&self) -> Option<&TextureView> {
        self.gbuffer.albedo_view()
    }
    pub fn normal_view(&self) -> Option<&TextureView> {
        self.gbuffer.normal_view()
    }
    pub fn material_view(&self) -> Option<&TextureView> {
        self.gbuffer.material_view()
    }
    pub fn raster_depth_view(&self) -> Option<&TextureView> {
        self.gbuffer.raster_depth_view()
    }
    pub fn combined_depth_view(&self) -> Option<&TextureView> {
        self.gbuffer.combined_depth_view()
    }
 
}



#[derive(ShaderType, Clone, Debug)]
pub struct RasterMaterialUniform {
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct RasterMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub base_color_texture: Option<Handle<Image>>,

    #[texture(2)]
    #[sampler(3)]
    pub normal_texture: Option<Handle<Image>>,

    #[texture(4)]
    #[sampler(5)]
    pub metallic_roughness_texture: Option<Handle<Image>>,

    #[uniform(6)]
    pub material_uniform: RasterMaterialUniform,
}
impl Material for RasterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/geom_internal.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Zeroable)]
pub struct FrameUniform {
    pub res: Vec2,
    pub frame_index: u32,
    pub sun_pos: Vec3,
}

#[derive(Resource)]
pub struct FrameBindLayout {
    layout: BindGroupLayoutDescriptor,
}
impl FrameBindLayout {
    pub fn new(layout: BindGroupLayoutDescriptor) -> Self {
        Self { layout }
    }
    pub fn get_layout(&self) -> &BindGroupLayoutDescriptor {
        &self.layout
    }
}

#[derive(Resource)]
pub struct Binder<T> {
    group: BindGroup,
    offsets: Vec<u32>,
    _marker: std::marker::PhantomData<T>,
}
impl<T> Binder<T> {
    pub fn new(group: BindGroup, offsets: Vec<u32>) -> Self {
        Self {
            group,
            offsets,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn new_empty(group: BindGroup) -> Self {
        Self::new(group, Vec::new())
    }
    pub fn get_group(&self) -> &BindGroup {
        &self.group
    }
    pub fn get_offsets(&self) -> &[u32] {
        &self.offsets.as_slice()
    }
}

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct PostLabel;


#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct RaymarchLabel;

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct RasterLabel;

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct LightingLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct BlitLabel;

pub struct Frame;
pub struct Raymarch;
pub struct Lighting;
pub struct Post;

pub struct Blit;


pub fn dispatch_size(dim: Vec4) -> UVec3 {
    uvec3(
        (dim.z.round() as u32 + 7) / 8,
        (dim.w.round() as u32 + 7) / 8,
        1,
    )
}

pub fn create_tex_desc<'w>(label: &'w str, format: TextureFormat, usage: TextureUsages, size:Extent3d)->TextureDescriptor<'w> {
    TextureDescriptor {
        label: Some(label),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
        mip_level_count: 1,
    }
}

pub fn create_tex_storage_desc<'w>(label: &'w str, format: TextureFormat, size:Extent3d)->TextureDescriptor<'w> {
    TextureDescriptor {
        label: Some(label),
        size,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
        mip_level_count: 1,
    }
}