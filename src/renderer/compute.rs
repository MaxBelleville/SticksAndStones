use std::{borrow::Cow, marker::PhantomData};

use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, ViewNode},
        render_resource::{
            BindGroupLayoutDescriptor, CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache
        },
        renderer::RenderContext,
    },
};

use super::utils;
use crate::window::WindowSettings;

pub trait ComputePassSpec: Send + Sync + 'static {
    /// Marker type used for the binder
    type Marker: Send + Sync + 'static;
    type Frame: Send + Sync + 'static;

    /// Shader path
    const SHADER_PATH: &'static str;

    /// Entry point
    const ENTRY: &'static str = "main";

    /// Build the bind group layout
    fn create_layout() -> BindGroupLayoutDescriptor;
}

#[derive(Resource)]
pub struct ComputePipelineRes<P: ComputePassSpec> {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
    _pipe_marker: PhantomData<P>,
}
impl<P: ComputePassSpec> ComputePipelineRes<P> {
    pub fn get_pipeline(&self) -> CachedComputePipelineId {
        self.pipeline
    }
    pub fn get_layout(&self) -> &BindGroupLayoutDescriptor {
        &self.layout
    }
}

pub fn init_pipeline<P: ComputePassSpec>(
    mut commands: Commands,
    frame_layout: Res<utils::FrameBindLayout>,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = P::create_layout();

    let shader = asset_server.load(P::SHADER_PATH);

    let compute_pipeline_descriptor =    ComputePipelineDescriptor {
        label: None,
        layout: vec![frame_layout.get_layout().clone(),layout.clone()],
        shader,
        shader_defs: vec![],
        entry_point: Some(Cow::from(P::ENTRY)),
        push_constant_ranges: vec![],
        zero_initialize_workgroup_memory: true,
    };

    let pipeline = pipeline_cache.queue_compute_pipeline(compute_pipeline_descriptor);

    commands.insert_resource(ComputePipelineRes::<P> {
        layout,
        pipeline,
        _pipe_marker: PhantomData,
    });
}

#[derive(Default)]
pub struct ComputeNode<P: ComputePassSpec> {
    _pass_marker: PhantomData<P>,
}

impl<P: ComputePassSpec> ViewNode for ComputeNode<P> {
    type ViewQuery = ();

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ComputePipelineRes<P>>();
        let settings = world.resource::<WindowSettings>();

        let Some(pipeline_obj) = pipeline_cache.get_compute_pipeline(pipeline.get_pipeline()) else {
            return Ok(());
        };
        let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());
        
        pass.set_pipeline(pipeline_obj);
        
        let frame_binders = world.resource::<utils::BindersRes<P::Frame>>();
        let pass_binders = world.resource::<utils::BindersRes<P::Marker>>();

        for (index,frame) in frame_binders.get_binders().iter().enumerate() {
            pass.set_bind_group(index as u32, frame.get_group(), frame.get_offsets());
        }
        let offset = frame_binders.get_len();

        for (index,binder) in pass_binders.get_binders().iter().enumerate() {
            pass.set_bind_group((offset + index) as u32, binder.get_group(), binder.get_offsets());
        }

        let size = utils::dispatch_size(settings.dim);
        pass.dispatch_workgroups(size.x, size.y, size.z);

        Ok(())
    }
}
