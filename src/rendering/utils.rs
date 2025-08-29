use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
    window::WindowMode,
};
use bytemuck::{Pod, Zeroable};

use super::{res, shared};
pub fn compute_pipeline_descriptor(
    shader: Handle<Shader>,
    entry_point: &str,
    bind_group_layouts: &Vec<BindGroupLayout>,
) -> ComputePipelineDescriptor {
    ComputePipelineDescriptor {
        label: None,
        layout: bind_group_layouts.clone(),
        shader,
        shader_defs: vec![],
        entry_point: Cow::from(entry_point.to_owned()),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: true,
    }
}

pub fn render_pipeline_descriptor_empty(
    shader: Handle<Shader>,
    frag_entry: &str,
    vert_entry: &str,
    bind_group_layouts: &Vec<BindGroupLayout>,
) -> RenderPipelineDescriptor {
    render_pipeline_descriptor(
        shader,
        frag_entry,
        vert_entry,
        vec![],
        vec![],
        CompareFunction::Always,
        bind_group_layouts,
    )
}

pub fn render_pipeline_descriptor(
    shader: Handle<Shader>,
    frag_entry: &str,
    vert_entry: &str,
    frag_targets: Vec<Option<ColorTargetState>>,
    buffers: Vec<VertexBufferLayout>,
    depth_compare: CompareFunction,
    bind_group_layouts: &Vec<BindGroupLayout>,
) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label: None,
        layout: bind_group_layouts.clone(),

        vertex: VertexState {
            shader: shader.clone(),
            entry_point: Cow::from(vert_entry.to_owned()), // vertex entry
            buffers,
            shader_defs: vec![],
        },

        fragment: Some(FragmentState {
            shader,
            entry_point: Cow::from(frag_entry.to_owned()), // fragment entry
            targets: frag_targets,
            shader_defs: vec![], // no color, only depth
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState::default(),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: true,
    }
}

pub fn run_compute_pass(
    render_context: &mut RenderContext,
    bind_groups: Vec<&BindGroup>,
    pipeline_cache: &PipelineCache,
    settings: shared::res::WindowManagerSettings,
    pipeline: CachedComputePipelineId,
) {
    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());
    for (index, bind_group) in bind_groups.iter().enumerate() {
        pass.set_bind_group(index as u32, *bind_group, &[]);
    }
    let pipeline = pipeline_cache.get_compute_pipeline(pipeline).unwrap();
    pass.set_pipeline(pipeline);

    pass.dispatch_workgroups(
        settings.dim.z as u32 / settings.workgroup_size,
        settings.dim.w as u32 / settings.workgroup_size,
        1,
    );
}

pub fn create_bind_group(
    label: &str,
    render_device: &RenderDevice,
    layout: &BindGroupLayout,
    targets: &res::GBufferTargets,
) -> BindGroup {
    render_device.create_bind_group(
        label,
        layout,
        &BindGroupEntries::sequential((
            &targets.albedo.as_ref().unwrap().default_view,
            &targets.normal.as_ref().unwrap().default_view,
            &targets.ray_depth.as_ref().unwrap().default_view,
        )),
    )
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Zeroable)]
pub struct EnviromentData {
    cam_rotx: Vec3,
    sunx: f32,
    cam_roty: Vec3,
    suny: f32,
    cam_rotz: Vec3,
    sunz: f32,
    cam_pos: Vec3,
    time: f32,
}
impl EnviromentData {
    pub fn new(atmo: &shared::res::AtmosphereData, rot: Mat3, cam_pos: Vec3) -> Self {
        Self {
            cam_rotx: rot.x_axis,
            sunx: atmo.sun_dir.x,
            cam_roty: rot.y_axis,
            suny: atmo.sun_dir.y,
            cam_rotz: rot.z_axis,
            sunz: atmo.sun_dir.z,
            cam_pos,
            time: atmo.time,
        }
    }
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy, Pod, Default, Zeroable)]
pub struct PushConstants {
    res: Vec2,
    fov_y_deg: f32, // 4 bytes (to align the Vec3)
    root_size: f32,
    root_min: Vec3,
    mode: f32, // 4 bytes (to bring total to 32)
}

impl PushConstants {
    pub fn new(
        res: &shared::res::WorldData,
        settings: &shared::res::WindowManagerSettings,
    ) -> Self {
        let mode = if settings.window_mode != WindowMode::Windowed {
            0.0
        } else {
            1.0
        };
        Self {
            res: vec2(settings.dim.z, settings.dim.w),
            root_size: res.root_size,
            root_min: res.root_min,
            fov_y_deg: 70.0,
            mode: mode,
        }
    }
}
