use std::collections::HashMap;
use bytemuck::NoUninit;
use simdnoise::NoiseBuilder;
use bevy_math::*;

use std::marker::PhantomData;
use std::mem::size_of;

#[derive(Default)]
pub struct NoiseFields {
    noise_fields: HashMap<String, NoiseField>,
}
impl NoiseFields {
    pub fn get(&self, key: String) -> Option<&NoiseField> {
        self.noise_fields.get(&key)
    }
}

#[derive(Default)]
pub struct NoiseField {
    frequency: f32,
    octaves: u8,
    gain: f32,
    lacunarity: f32,
    mult: f32,
}

impl NoiseField {
    pub fn new(frequency:f32,octaves:u8,gain:f32,lacunarity:f32)->Self {
        Self {
            frequency,
            octaves,
            gain,
            lacunarity,
            mult: 1.0,
        }
    }
}

impl NoiseField {
    pub fn sample_fbm_2d(&self, coord: Vec2, seed: i32) -> f32 {
        let val = NoiseBuilder::fbm_2d_offset(coord.x, 1, coord.y, 1)
            .with_seed(seed)
            .with_freq(self.frequency)
            .with_octaves(self.octaves)
            .with_gain(self.gain)
            .with_lacunarity(self.lacunarity)
            .generate_scaled(0.0, 1.0);
        val[0] * self.mult
    }
  
}

pub struct LinearAllocator<T> {
    storage: Vec<T>,
}

impl<T> LinearAllocator<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: Vec::with_capacity(capacity),
        }
    }

    pub fn allocate(&mut self, value: T) -> usize {
        let index = self.storage.len();
        self.storage.push(value);
        index
    }

   
    pub fn get(&self, index: usize) -> &T {
        &self.storage[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.storage[index]
    }

    pub fn clear(&mut self) {
        self.storage.clear();
    }
    pub fn slice(&self, start: usize, count: usize) -> &[T] {
        &self.storage[start..start + count]
    }

    pub fn as_slice(&self) -> &[T] {
        &self.storage
    }
}

#[derive(Default,Clone,Copy)]
pub struct GpuBuffer<T> {
    /// Backend-specific handle (VkBuffer, CUdeviceptr, wgpu::Buffer, etc.)
    handle: GpuBufferHandle,

    /// Number of elements allocated on GPU
    capacity: usize,

    /// Usage flags (storage, transfer, etc.)
    usage: GpuBufferUsage,

    _marker: PhantomData<T>,
}

#[derive(Clone, Copy,Default)]
pub struct GpuBufferHandle {
    raw: u64, // opaque backend handle
}

#[derive(Clone, Copy,Default)]
pub struct GpuBufferUsage {
    pub storage: bool,
    pub transfer_src: bool,
    pub transfer_dst: bool,
}

pub trait GpuDevice {
    fn create_buffer(&self, size: usize, usage: GpuBufferUsage) -> GpuBufferHandle;
    fn destroy_buffer(&self, handle: GpuBufferHandle);
    fn upload_buffer(&self, handle: GpuBufferHandle, offset: usize, data: &[u8], size: usize);
    fn copy_buffer(&self, src: GpuBufferHandle, dst: GpuBufferHandle, size: usize);
}

impl<T> GpuBuffer<T> {
    pub fn new(
        device: &dyn GpuDevice,
        capacity: usize,
        usage: GpuBufferUsage,
    ) -> Self {
        let size_bytes = capacity * size_of::<T>();

        let handle = device.create_buffer(size_bytes, usage);

        Self {
            handle,
            capacity,
            usage,
            _marker: PhantomData,
        }
    }

    pub fn destroy(self, device: &dyn GpuDevice) {
        device.destroy_buffer(self.handle);
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn handle(&self) -> GpuBufferHandle {
        self.handle
    }
}

impl<T> GpuBuffer<T> {
    pub fn upload_range(
        &self,
        device: &dyn GpuDevice,
        offset: usize,
        data: &[T],
    )
    where T: NoUninit
     {
        debug_assert!(offset + data.len() <= self.capacity);

        let byte_offset = offset * size_of::<T>();
        let byte_size = data.len() * size_of::<T>();

        device.upload_buffer(
            self.handle,
            byte_offset,
            bytemuck::cast_slice(data),
            byte_size,
        );
    }
}

impl<T> GpuBuffer<T> {
    pub fn grow(
        &mut self,
        device: &dyn GpuDevice,
        new_capacity: usize,
        copy_existing: bool,
    ) {
        assert!(new_capacity > self.capacity);

        let new_size = new_capacity * size_of::<T>();
        let new_handle = device.create_buffer(new_size, self.usage);

        if copy_existing {
            let copy_bytes = self.capacity * size_of::<T>();
            device.copy_buffer(
                self.handle,
                new_handle,
                copy_bytes,
            );
        }

        device.destroy_buffer(self.handle);

        self.handle = new_handle;
        self.capacity = new_capacity;
    }
}

#[derive(Default,Clone,Copy)]
pub struct Range<T> {
    start: T,
    end: T,
}
impl<T> Range<T> 
    where T: Clone {
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
    pub fn set_range(&mut self, start: T, end: T) {
        self.start = start;
        self.end = end;
    }
    pub fn set_start(&mut self, start: T) {
        self.start = start;
    }
    pub fn set_end(&mut self, end: T) {
        self.end = end;
    }
    pub fn get_range(&self) -> (T,T) {
        (self.start.clone(), self.end.clone())
    }
    pub fn start(&self) -> T {
        self.start.clone()
    }
    pub fn end(&self) -> T {
        self.end.clone()
    }
}
