use std::hash::{Hash, Hasher};

use bytemuck::{Pod, Zeroable};

pub const NO_LEAF: u32 = u32::MAX; //If leaf and this is set then the area is empty.

#[inline]
pub fn get_compact_idx(indices2b: &[u8; 16], i: usize) -> u8 {
    let byte = i / 4;
    let shift = (i % 4) * 2;
    (indices2b[byte] >> shift) & 0b11
}

#[inline]
pub fn set_compact_idx(indices2b: &mut [u8; 16], i: usize, val: u8) {
    let byte = i / 4;
    let shift = (i % 4) * 2;
    indices2b[byte] &= !(0b11 << shift);
    indices2b[byte] |= (val & 0b11) << shift;
}

#[derive(Debug, PartialEq)]
pub enum LeafType {
    Uniform(LeafUniformBlob),
    Compact(LeafCompactArrayBlob),
    Raw64(LeafRaw64Blob),
    Empty,
}

impl Hash for LeafType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            LeafType::Uniform(blob) => blob.mat.hash(state),
            LeafType::Compact(blob) => {
                blob.occ_mask.hash(state);
                blob.palette.hash(state);
                blob.palette_size.hash(state);
                blob.indices2b.hash(state);
            }
            LeafType::Raw64(blob) => blob.voxels.hash(state),
            LeafType::Empty => 0u8.hash(state),
        }
    }
}

impl LeafType {
    pub fn classify(voxels: [u8; 64]) -> Self {
        let mut is_uniform: bool = true;
        let first = voxels[0];
        for voxel in voxels {
            if first != voxel {
                is_uniform = false;
                break;
            }
        }
        if is_uniform && first == 0 {
            return LeafType::Empty;
        } else if is_uniform {
            return LeafType::Uniform(LeafUniformBlob::new(first));
        }
        // Count unique non-air materials
        let mut palette = [0u8; 4];
        let mut palette_size = 0;
        let mut indices = [0u8; 64];
        let mut unique = std::collections::HashMap::new();

        for (i, &mat) in voxels.iter().enumerate() {
            if mat == 0 {
                continue;
            } // skip air
            if let Some(&idx) = unique.get(&mat) {
                indices[i] = idx as u8;
            } else {
                if palette_size >= 4 {
                    // Too many unique → Raw64
                    return LeafType::Raw64(LeafRaw64Blob::new(voxels));
                }
                palette[palette_size] = mat;
                unique.insert(mat, palette_size);
                indices[i] = palette_size as u8;
                palette_size += 1;
            }
        }

        // Otherwise compact representation
        let mut occ_mask = 0u64;
        let mut indices2b = [0u8; 16];
        for i in 0..64 {
            if voxels[i] != 0 {
                occ_mask |= 1 << i;
                let palette_idx = indices[i] & 0b11;
                set_compact_idx(&mut indices2b, i, palette_idx);
            }
        }

        LeafType::Compact(LeafCompactArrayBlob::new(
            occ_mask,
            palette,
            palette_size as u8,
            indices2b,
        ))
    }

    pub fn update_voxel(&mut self, idx: usize, mat: u8) -> Self {
        match self {
            LeafType::Empty => {
                if mat == 0 {
                    LeafType::Empty
                } else {
                    let mut voxels = [0u8; 64];
                    voxels[idx] = mat;
                    LeafType::Compact(LeafCompactArrayBlob::from_single(idx, mat))
                }
            }
            LeafType::Uniform(blob) => {
                if mat == 0 {
                    // Air replaces uniform → compact with single voxel missing
                    let mut compact = LeafCompactArrayBlob::from_uniform(blob.mat);
                    compact.remove(idx);
                    LeafType::Compact(compact)
                } else if mat == blob.mat {
                    LeafType::Uniform(LeafUniformBlob::new(blob.mat)) // same uniform → no change
                } else {
                    // Break uniform → compact with 2 materials
                    let mut compact = LeafCompactArrayBlob::from_uniform(blob.mat);
                    compact.insert(idx, mat);
                    LeafType::Compact(compact)
                }
            }
            LeafType::Compact(blob) => {
                if mat == 0 {
                    blob.remove(idx);
                    if blob.occ_mask == 0 {
                        LeafType::Empty
                    } else {
                        LeafType::Compact(*blob)
                    }
                } else if blob.can_insert(mat) {
                    blob.insert(idx, mat);
                    if blob.occ_mask == u64::MAX && blob.palette_size == 1 {
                        // Upgrade to uniform if cube is fully filled with same mat
                        LeafType::Uniform(LeafUniformBlob::new(mat))
                    } else {
                        LeafType::Compact(*blob)
                    }
                } else {
                    let mut raw = LeafRaw64Blob::from_compact(&blob);
                    raw.voxels[idx] = mat;
                    LeafType::Raw64(raw)
                }
            }
            LeafType::Raw64(blob) => {
                blob.voxels[idx] = mat;
                LeafType::Raw64(*blob)
            }
        }
    }

    pub fn get_voxel(&self, idx: usize) -> u8 {
        match self {
            LeafType::Empty => 0,
            LeafType::Uniform(blob) => blob.mat,
            LeafType::Compact(blob) => {
                let compact_idx = get_compact_idx(&blob.indices2b, idx) as usize;
                blob.palette[compact_idx]
            }
            LeafType::Raw64(blob) => blob.voxels[idx],
        }
    }
}

#[derive(Debug)]
pub struct LeafPool {
    leaves: Vec<LeafType>, // easy for CPU/DAGify
    free_list: Vec<usize>,
}
impl LeafPool {
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            free_list: Vec::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    pub fn push_leaf(&mut self, leaf_type: LeafType) -> usize {
        let idx = self.leaves.len() as usize;
        if leaf_type == LeafType::Empty {
            return NO_LEAF as usize;
        }
        if let Some(base) = self.free_list.pop() {
            self.leaves[base as usize] = leaf_type;
            return base;
        }

        self.leaves.push(leaf_type);
        idx
    }

    pub fn update_leaf(&mut self, leaf_idx: usize, leaf_type: LeafType) {
        self.leaves[leaf_idx] = leaf_type;
    }

    pub fn free_leaf(&mut self, base: usize) {
        if let Some(slot) = self.leaves.get_mut(base) {
            *slot = LeafType::Empty;
            self.free_list.push(base);
        }
    }

    pub fn get_type(&self, base: usize) -> &LeafType {
        if let Some(slot) = self.leaves.get(base) {
            return slot;
        }
        &LeafType::Empty
    }

    pub fn update_voxel(&mut self, base: usize, child_base: usize, mat: u8) -> LeafType {
        if let Some(slot) = self.leaves.get_mut(base) {
            return slot.update_voxel(child_base, mat);
        }
        LeafType::Empty
    }

    pub fn is_uniform(&self, base: usize, extent: usize) -> (u8, usize) {
        // Inspect leaf blob (via leaves_pool) //Should probably
        match self.get_type(base) {
            LeafType::Uniform(b) => {
                let mat = b.mat;
                (mat, extent * extent * extent)
            }
            LeafType::Compact(b) => {
                let palette_size = b.palette_size as usize;
                if palette_size == 0 {
                    (0, 0)
                } else if palette_size == 1 {
                    // popcount on occ_mask:
                    let occ = b.occ_mask;
                    let count = occ.count_ones() as usize;
                    (b.palette[0], count)
                } else {
                    (0, 0)
                }
            }
            LeafType::Raw64(_b) => {
                // we ignore the prefill data as raw would only happen
                (0, 0)
            }
            LeafType::Empty => (0, 0),
        }
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for leaf_type in &self.leaves {
            match leaf_type {
                LeafType::Uniform(b) => bytes.extend_from_slice(bytemuck::bytes_of(b)),
                LeafType::Compact(b) => bytes.extend_from_slice(bytemuck::bytes_of(b)),
                LeafType::Raw64(b) => bytes.extend_from_slice(bytemuck::bytes_of(b)),
                LeafType::Empty => {}
            }
        }
        bytes
    }

    pub fn get_voxel(&self, base: usize, child_base: usize) -> u8 {
        if let Some(slot) = self.leaves.get(base) {
            return slot.get_voxel(child_base);
        }
        0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct LeafHeader {
    pub kind: u8, // 0 = uniform, 1 = array
    pub size: u8, // total bytes including header (for iteration / validation)
    pub _reserved: [u8; 2],
}
impl LeafHeader {
    pub fn new(kind: u8, size: u8) -> Self {
        Self {
            kind,
            size,
            _reserved: [0; 2],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct LeafUniformBlob {
    pub header: LeafHeader, // kind=0, size=8
    pub mat: u8,
    pub _pad: [u8; 3], // align to 8
}

impl LeafUniformBlob {
    pub fn new(mat: u8) -> Self {
        Self {
            header: LeafHeader::new(0, 8),
            mat,
            _pad: [0; 3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct LeafCompactArrayBlob {
    pub header: LeafHeader,    // 4
    pub _pad_header: [u8; 4],  // explicit pad → ensures occ_mask is aligned
    pub occ_mask: u64,         // 8
    pub palette: [u8; 4],      // 4
    pub palette_size: u8,      // 1
    pub _pad_palette: [u8; 3], // pad to 8
    pub indices2b: [u8; 16],   // 16
    pub _pad_end: [u8; 8],     // pad total size to multiple of 8
}

impl LeafCompactArrayBlob {
    pub fn new(occ_mask: u64, palette: [u8; 4], palette_size: u8, indices2b: [u8; 16]) -> Self {
        Self {
            header: LeafHeader::new(1, 48),
            _pad_header: [0; 4],
            occ_mask,
            palette,
            palette_size,
            _pad_palette: [0; 3],
            indices2b,
            _pad_end: [0; 8],
        }
    }

    pub fn from_single(idx: usize, mat: u8) -> Self {
        assert!(idx < 64);

        // Occ mask has just one bit set
        let occ_mask = 1u64 << idx;

        // Palette with one entry
        let mut palette = [0u8; 4];
        palette[0] = mat;

        // Indices2b encodes up to 4 voxels per byte
        let mut indices2b = [0u8; 16];
        let byte = idx / 4;
        indices2b[byte] |= 0; // palette index = 0 (first entry)

        Self {
            header: LeafHeader::new(1, 48),
            _pad_header: [0; 4],
            occ_mask,
            palette,
            palette_size: 1,
            _pad_palette: [0; 3],
            indices2b,
            _pad_end: [0; 8],
        }
    }

    pub fn from_uniform(mat: u8) -> Self {
        let mut palette = [0u8; 4];
        palette[0] = mat;

        let indices2b = [0u8; 16];

        Self {
            header: LeafHeader::new(1, 64),
            _pad_header: [0; 4],
            occ_mask: u64::MAX, // all 64 bits set
            palette,
            palette_size: 1,
            _pad_palette: [0; 3],
            indices2b,
            _pad_end: [0; 8],
        }
    }

    pub fn can_insert(self, mat: u8) -> bool {
        for pal_mat in self.palette {
            if pal_mat == mat || pal_mat == 0 {
                return true;
            }
        }
        false
    }

    pub fn insert(&mut self, idx: usize, mat: u8) {
        assert!(mat != 0 && idx < 64);

        // Check if material already exists in palette
        let mut palette_idx = None;
        for (i, &p) in self
            .palette
            .iter()
            .enumerate()
            .take(self.palette_size as usize)
        {
            if p == mat {
                palette_idx = Some(i);
                break;
            }
        }

        // If not found, insert into palette
        if palette_idx.is_none() {
            assert!(self.palette_size < 4, "Palette overflow");
            palette_idx = Some(self.palette_size as usize);
            self.palette[self.palette_size as usize] = mat;
            self.palette_size += 1;
        }

        let palette_idx = palette_idx.unwrap() as u8;

        // Update occ_mask
        self.occ_mask |= 1 << idx;

        // Update indices2b (2 bits per voxel)
        set_compact_idx(&mut self.indices2b, idx, palette_idx);
    }

    pub fn remove(&mut self, idx: usize) {
        assert!(idx < 64);

        // Clear occ_mask
        self.occ_mask &= !(1 << idx);

        // Get old palette index from indices2b
        let byte = idx / 4;
        let shift = (idx % 4) * 2;
        let old_palette_idx = (self.indices2b[byte] >> shift) & 0b11;

        // Clear indices2b
        self.indices2b[byte] &= !(0b11 << shift);

        // If this palette entry is unused anywhere → shrink palette
        if !self.is_palette_index_used(old_palette_idx as usize) {
            self.remove_palette_entry(old_palette_idx as usize);
        }
    }

    fn is_palette_index_used(&self, idx: usize) -> bool {
        for byte in &self.indices2b {
            for bitshift in [0, 2, 4, 6] {
                if ((*byte >> bitshift) & 0b11) == idx as u8 {
                    return true;
                }
            }
        }
        false
    }

    fn remove_palette_entry(&mut self, idx: usize) {
        // Shift palettes down
        for i in idx..(self.palette_size as usize - 1) {
            self.palette[i] = self.palette[i + 1];
        }
        self.palette[(self.palette_size - 1) as usize] = 0;
        self.palette_size -= 1;

        // Fix up indices2b (decrement any entry > idx)
        for byte in &mut self.indices2b {
            for shift in [0, 2, 4, 6] {
                let val = (*byte >> shift) & 0b11;
                if val > idx as u8 {
                    *byte &= !(0b11 << shift);
                    *byte |= (val - 1) << shift;
                }
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct LeafRaw64Blob {
    pub header: LeafHeader, // kind=2, size=72
    pub voxels: [u8; 64],   // direct 1 byte per voxel
    pub _pad_end: [u8; 4],  // pad to 72 (multiple of 8)
}
impl LeafRaw64Blob {
    pub fn new(voxels: [u8; 64]) -> Self {
        Self {
            header: LeafHeader::new(2, 72),
            voxels,
            _pad_end: [0; 4],
        }
    }

    pub fn from_compact(compact: &LeafCompactArrayBlob) -> Self {
        let mut voxels = [0u8; 64];

        // Fill each voxel from palette+indices if occupied
        for i in 0..64 {
            if ((compact.occ_mask >> i) & 1) != 0 {
                let idx2 = get_compact_idx(&compact.indices2b, i) as usize;
                // idx2 must be within palette_size in well-formed blobs
                voxels[i] = compact.palette[idx2];
            } else {
                voxels[i] = 0;
            }
        }

        Self {
            header: LeafHeader::new(2, 72),
            voxels,
            _pad_end: [0; 4],
        }
    }
}
