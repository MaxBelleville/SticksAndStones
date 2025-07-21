use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{render_resource::*, renderer::RenderContext},
};

use super::shared;
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

pub fn run_compute_pass(
    render_context: &mut RenderContext,
    out_bind: &BindGroup,
    shared_bind: &BindGroup,
    pipeline_cache: &PipelineCache,
    settings: shared::res::WindowManagerSettings,
    pipeline: CachedComputePipelineId,
) {
    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());

    pass.set_bind_group(0, out_bind, &[]);

     pass.set_bind_group(1, shared_bind, &[]);

    let pipeline = pipeline_cache.get_compute_pipeline(pipeline).unwrap();
    pass.set_pipeline(pipeline);

    pass.dispatch_workgroups(
        settings.dim.z as u32 / settings.workgroup_size,
        settings.dim.w as u32 / settings.workgroup_size,
        1,
    );
}
