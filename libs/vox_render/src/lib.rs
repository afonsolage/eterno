use serde::Deserialize;
use serde::Serialize;
use vox::*;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct FacesOcclusion(u8);

const FULL_OCCLUDED_MASK: u8 = 0b0011_1111;

impl FacesOcclusion {
    pub fn set_all(&mut self, occluded: bool) {
        if occluded {
            self.0 = FULL_OCCLUDED_MASK;
        } else {
            self.0 = 0;
        }
    }

    pub fn is_fully_occluded(&self) -> bool {
        self.0 & FULL_OCCLUDED_MASK == FULL_OCCLUDED_MASK
    }

    pub fn is_occluded(&self, side: voxel::Side) -> bool {
        let mask = 1 << side as usize;
        self.0 & mask == mask
    }

    pub fn set(&mut self, side: voxel::Side, occluded: bool) {
        let mask = 1 << side as usize;
        if occluded {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }
}

impl From<[bool; 6]> for FacesOcclusion {
    fn from(v: [bool; 6]) -> Self {
        let mut result = Self::default();

        for side in voxel::SIDES {
            result.set(side, v[side as usize]);
        }

        result
    }
}

impl vox::chunk::ChunkStorageType for FacesOcclusion {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_occlusion() {
        let mut occlusion = FacesOcclusion::default();
        assert!(!occlusion.is_fully_occluded());

        for side in voxel::SIDES {
            assert!(!occlusion.is_occluded(side));
        }

        occlusion.set(voxel::Side::Up, true);
        assert!(occlusion.is_occluded(voxel::Side::Up));

        occlusion.set(voxel::Side::Back, true);
        assert!(occlusion.is_occluded(voxel::Side::Back));

        for side in voxel::SIDES {
            occlusion.set(side, true);
        }

        assert!(occlusion.is_fully_occluded());

        for side in voxel::SIDES {
            assert!(occlusion.is_occluded(side));
        }

        occlusion.set(voxel::Side::Back, false);
        assert!(!occlusion.is_occluded(voxel::Side::Back));

        for side in voxel::SIDES {
            occlusion.set(side, false);
        }

        assert!(!occlusion.is_fully_occluded());

        for side in voxel::SIDES {
            assert!(!occlusion.is_occluded(side));
        }
    }
}
