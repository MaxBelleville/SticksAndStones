use std::collections::HashMap;
use simdnoise::NoiseBuilder;
use bevy_math::*;

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

    #[inline]
    pub fn allocate(&mut self, value: T) -> u32 {
        let index = self.storage.len() as u32;
        self.storage.push(value);
        index
    }

    #[inline]
    pub fn get(&self, index: u32) -> &T {
        &self.storage[index as usize]
    }

    #[inline]
    pub fn get_mut(&mut self, index: u32) -> &mut T {
        &mut self.storage[index as usize]
    }

    #[inline]
    pub fn clear(&mut self) {
        self.storage.clear();
    }

    pub fn as_slice(&self) -> &[T] {
        &self.storage
    }
}
