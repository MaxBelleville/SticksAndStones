use super::{
    BRICKS_PER_BRICKMAP, BrickHubCell, CellID, MaterialID, VOXELS_PER_BRICK, allocators, brick,
    device,
};
use bevy_math::*;
use sns_core::prelude::{GpuDevice, Range};
use std::collections::HashMap;

pub struct GpuMigrationTask {
    pub id: CellID,
    pub old_alloc: allocators::CellAllocation,
    pub new_alloc: allocators::CellAllocation,
}

pub struct UploadBatch {
    pub brick_dst: usize,
    pub material_dst: usize,
    pub index_dst: usize,

    pub brick_src: UploadSource,
    pub material_src: UploadSource,
    pub index_src: UploadSource,
}

pub enum UploadSource {
    Bricks(Vec<brick::Brick>),
    Materials(Vec<MaterialID>),
    Indices(Vec<u32>),

    GpuRange { src_offset: usize, len: usize },
}

#[derive(Default)]
pub struct DeviceScheduler {
    migration_queue: Vec<GpuMigrationTask>,
    upload_queue: Vec<UploadBatch>,
}

impl DeviceScheduler {
    pub fn flush_deltas(
        &mut self,
        batch: device::DeltaBatch,
        cells: &HashMap<IVec3, BrickHubCell>,
        allocations: &allocators::DeviceAllocator,
        buffers: &mut device::Buffers,
        gpu: &dyn GpuDevice,
    ) {
        self.upload_ranges(
            &batch.get_ranges(device::StorageType::Brick),
            device::StorageType::Brick,
            cells,
            allocations,
            buffers,
            gpu,
        );
        self.upload_ranges(
            &batch.get_ranges(device::StorageType::Material),
            device::StorageType::Material,
            cells,
            allocations,
            buffers,
            gpu,
        );
        self.upload_ranges(
            &batch.get_ranges(device::StorageType::Indices),
            device::StorageType::Indices,
            cells,
            allocations,
            buffers,
            gpu,
        );
    }
    pub fn upload_ranges(
        &mut self,
        ranges: &Vec<device::CellRange>,
        storage_type: device::StorageType,
        cells: &HashMap<IVec3, BrickHubCell>,
        allocations: &allocators::DeviceAllocator,
        buffer: &mut device::Buffers,
        gpu: &dyn GpuDevice,
    ) {
        if ranges.is_empty() {
            return;
        }
        for cell_range in ranges {
            let id = cell_range.get_id();

            // 1. Get allocation
            let alloc = match allocations.get(id) {
                Some(a) => a,
                None => continue, // cell not resident
            };
            // 2. Compute device base
            let device_base = match storage_type {
                device::StorageType::Brick => alloc.get_brick_base(),
                device::StorageType::Material => alloc.get_material_base(),
                device::StorageType::Indices => alloc.get_index_base(),
            };

            let local_range = cell_range.get_range();
            let device_offset = device_base + local_range.start();

            // 3. Get CPU slice
            let cell_ref = match cells.get(&id) {
                Some(c) => c,
                None => continue,
            };

            match storage_type {
                device::StorageType::Brick => {
                    let data = cell_ref.get_brickmap().get_brick(local_range.clone());
                    buffer
                        .get_brick_mut()
                        .upload_range(gpu, device_offset, data);
                }
                device::StorageType::Material => {
                    let data = cell_ref.get_brickmap().get_material(local_range.clone());
                    buffer
                        .get_material_mut()
                        .upload_range(gpu, device_offset, data);
                }
                device::StorageType::Indices => {
                    let data = cell_ref.get_brickmap().get_indices(local_range.clone());
                    buffer
                        .get_index_mut()
                        .upload_range(gpu, device_offset, data);
                }
            };
        }
    }
    pub fn resolve_migrations(&mut self, cells: &HashMap<IVec3, BrickHubCell>) {
        for task in self.migration_queue.drain(..) {
            let Some(cell) = cells.get(&task.id) else {
                continue;
            };

            let bricks = cell
                .get_brickmap()
                .get_brick(Range::new(0, BRICKS_PER_BRICKMAP))
                .to_vec();
            let materials = cell
                .get_brickmap()
                .get_material(Range::new(0, BRICKS_PER_BRICKMAP * VOXELS_PER_BRICK))
                .to_vec();
            let indices = cell
                .get_brickmap()
                .get_indices(Range::new(0, BRICKS_PER_BRICKMAP))
                .to_vec();
            self.upload_queue.push(UploadBatch {
                brick_dst: task.new_alloc.get_brick_base(),
                material_dst: task.new_alloc.get_material_base(),
                index_dst: task.new_alloc.get_index_base(),

                brick_src: UploadSource::Bricks(bricks),
                material_src: UploadSource::Materials(materials),
                index_src: UploadSource::Indices(indices),
            });
        }
    }

    pub fn execute_uploads(&mut self, gpu: &dyn GpuDevice, buffers: &mut device::Buffers) {
        for batch in self.upload_queue.drain(..) {
            match batch.brick_src {
                UploadSource::Bricks(data) => {
                    buffers
                        .get_brick_mut()
                        .upload_range(gpu, batch.brick_dst, &data);
                }
                UploadSource::Materials(data) => {
                    buffers
                        .get_material_mut()
                        .upload_range(gpu, batch.material_dst, &data);
                }
                UploadSource::Indices(data) => {
                    buffers
                        .get_index_mut()
                        .upload_range(gpu, batch.index_dst, &data);
                }
                UploadSource::GpuRange { src_offset, len } => {
                    buffers
                        .get_brick_mut()
                        .copy_range(gpu, src_offset, batch.brick_dst, len);
                }
            }
        }
    }

    pub fn enqueue(&mut self, clone: GpuMigrationTask) {
        self.migration_queue.push(clone);
    }
    pub fn deenqueue(&mut self) -> Option<GpuMigrationTask> {
        self.migration_queue.pop()
    }
}
