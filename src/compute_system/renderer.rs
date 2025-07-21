use core::panic;

use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_resource::ExtractResourcePlugin,
        render_asset::RenderAssets,
        render_graph::{RenderGraph, RenderLabel},
        render_resource::{BindGroupEntries, BufferInitDescriptor, BufferUsages},
        renderer::RenderDevice,
        texture::GpuImage,
        view::{ExtractedView, ViewDepthTexture},
    },
};

use super::{
    ComputeBindGroups, ComputeRenderImages, EnviromentData, PushConstants,
    node::ComputeRendererNode, render_pipe::ComputeRenderPipeline, shared,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ComputeRendererLabel;

pub struct ComputeRendererPlugin;

impl Plugin for ComputeRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<ComputeRenderImages>::default());
        app.add_plugins(ExtractResourcePlugin::<shared::res::SVTPushConsts>::default());
        app.add_plugins(ExtractResourcePlugin::<shared::res::WindowManagerSettings>::default());
        app.add_plugins(ExtractResourcePlugin::<shared::res::AtmosphereUniformData>::default());

        //app.add_plugins(ExtractResourcePlugin::<>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(ComputeRendererLabel, ComputeRendererNode::default());
        render_graph.add_node_edge(ComputeRendererLabel, bevy::render::graph::CameraDriverLabel);
    }
    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ComputeRenderPipeline>();
    }
}

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ComputeRenderPipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    views: Query<(&ExtractedView, &ViewDepthTexture)>,
    compute_images: Res<ComputeRenderImages>,
    settings: Res<shared::res::WindowManagerSettings>,
    push_res: Res<shared::res::SVTPushConsts>,
    atmo_data: Res<shared::res::AtmosphereUniformData>,
    render_device: Res<RenderDevice>,
) {
    if let Ok((view, depth_texture)) = views.single() {
        //Prepares buffers will be used to bind data later.
        
        let cam_pos = view.world_from_view.translation();
        let rot = Mat3::from_quat(view.world_from_view.rotation());


        let camera_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("EnviromentData"),
            contents: bytemuck::bytes_of(&EnviromentData {
                cam_rotx: rot.x_axis,
                cam_roty: rot.y_axis,
                cam_rotz: rot.z_axis,
                cam_pos,
                time: atmo_data.time,
                sunx: atmo_data.sun_dir.x,
                suny: atmo_data.sun_dir.y,
                sunz: atmo_data.sun_dir.z,
            }),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let push_constants = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("PushContants"),
            contents: bytemuck::bytes_of(&PushConstants::new(
                push_res.into_inner(),
                settings.into_inner(),
            )),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let int_data: Vec<i32> = vec![1, 2, 3, 4, 5];
        let svt_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("SVTArray"),
            contents: bytemuck::cast_slice(&int_data),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        //Sets up render frame bind group, will switch between gpu images every frame to allow for compute buffering.
        let view_texture = gpu_images.get(&compute_images.render_texture).unwrap();
        let view_buffer = gpu_images.get(&compute_images.render_buffer).unwrap();

        let bind_group_out_0 = render_device.create_bind_group(
            "out_0",
            &pipeline.bind_group_layouts[0],
            &BindGroupEntries::sequential((&view_texture.texture_view, &view_buffer.texture_view)),
        );
        let bind_group_out_1 = render_device.create_bind_group(
            "out_1",
            &pipeline.bind_group_layouts[0],
            &BindGroupEntries::sequential((&view_buffer.texture_view, &view_texture.texture_view)),
        );

        //Loads depth, camera, "push consts" and svt buffer data.
        let bind_group_shared = render_device.create_bind_group(
            "shared_bind",
            &pipeline.bind_group_layouts[1],
            &BindGroupEntries::sequential((
                depth_texture.view(),
                camera_buffer.as_entire_binding(),
                svt_buffer.as_entire_binding(),
                push_constants.as_entire_binding(),
            )),
        );

        commands.insert_resource(ComputeBindGroups {
            output_frames: [bind_group_out_0, bind_group_out_1],
            shared: bind_group_shared,
        });
    } else {
        panic!("View not found");
    }
}
