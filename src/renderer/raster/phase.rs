use std::ops::Range;

use bevy::{
    math::FloatOrd, pbr::{DrawMesh, SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup, SetMeshViewEmptyBindGroup}, prelude::*, render::{
        render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem, PhaseItemExtraIndex, SetItemPipeline, SortedPhaseItem
        },
        render_resource::CachedRenderPipelineId,
        sync_world::MainEntity,
    }
};

pub type DrawGBufferMeshCommands = (
    SetItemPipeline,
    // This will set the view bindings in group 0
    SetMeshViewBindGroup<0>,
    // This will set an empty bind group in group 1
    SetMeshViewEmptyBindGroup<1>,
    // This will set the mesh bindings in group 2
    SetMeshBindGroup<2>,
    SetMaterialBindGroup<3>,
    // This will draw the mesh
    DrawMesh,
);

pub struct RasterGBuffer3D {
    pub sort_key: FloatOrd,
    pub entity: (Entity, MainEntity),
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
    /// Whether the mesh in question is indexed (uses an index buffer in
    /// addition to its vertex buffer).
    pub indexed: bool,
}

impl PhaseItem for RasterGBuffer3D {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for RasterGBuffer3D {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // bevy normally uses radsort instead of the std slice::sort_by_key
        // radsort is a stable radix sort that performed better than `slice::sort_by_key` or `slice::sort_unstable_by_key`.
        // Since it is not re-exported by bevy, we just use the std sort for the purpose of the example
        items.sort_by_key(SortedPhaseItem::sort_key);
    }

    #[inline]
    fn indexed(&self) -> bool {
        self.indexed
    }
}

impl CachedRenderPipelinePhaseItem for RasterGBuffer3D {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}
