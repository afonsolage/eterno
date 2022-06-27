// How to handle genesis and voxel update?

/*
    Keep a separated world updating voxel and receiving commands;

    Updates are handled via commands.
    How to handle queries?
        a. using requests/respose in a async way.
            - Pro: Have a better memory efficiency, since there will be only one copy of world at any point in time;
            - Con: Add a complexity when quering multiple entities;
        b. using systems requesting a copy of world.
            -

*/

use bevy::prelude::*;

use std::collections::HashSet;

use crate::chunk;
use crate::math;
use crate::voxel;
use crate::world::VoxWorld;

fn update_voxel(
    world: &mut VoxWorld,
    local: IVec3,
    voxels: &[(IVec3, voxel::Kind)],
) -> HashSet<IVec3> {
    trace!("Updating chunk {} values {:?}", local, voxels);
    let mut dirty_chunks = HashSet::default();

    if let Some(chunk) = world.get_mut(local) {
        for (voxel, kind) in voxels {
            chunk.set(*voxel, *kind);

            if chunk::is_at_bounds(*voxel) {
                let neighbor_dir = chunk::get_boundary_dir(*voxel);
                for unit_dir in math::to_unit_dir(neighbor_dir) {
                    let neighbor = unit_dir + local;
                    dirty_chunks.insert(neighbor);
                }
            }
        }

        dirty_chunks.insert(local);
    } else {
        warn!("Failed to set voxel. Chunk {} wasn't found.", local);
    }

    dirty_chunks
}

fn unload_chunk(world: &mut VoxWorld, local: IVec3) -> HashSet<IVec3> {
    let mut dirty_chunks = HashSet::default();

    if world.remove(local).is_none() {
        warn!("Trying to unload non-existing chunk {}", local);
    } else {
        dirty_chunks.extend(voxel::SIDES.map(|s| s.dir() + local))
    }

    dirty_chunks
}

fn load_chunk(world: &mut VoxWorld, local: IVec3) -> HashSet<IVec3> {
    let path = cache::local_path(local);

    let chunk = if path.exists() {
        cache::load(&path)
    } else {
        cache::generate(local)
    };

    world.add(local, chunk);

    voxel::SIDES
        .iter()
        .map(|s| s.dir() + local)
        .chain(std::iter::once(local))
        .collect()
}

fn update_chunk(world: &mut VoxWorld, local: IVec3) -> bool {
    if world.get(local).is_some() {
        world.update_neighborhood(local);
        true
    } else {
        false
    }
}

/**
    Chunk genesis caching related code
 */
mod cache {
    use super::*;

    use bracket_noise::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::path::Path;
    use std::path::PathBuf;

    const CACHE_PATH: &str = "cache/chunks";
    const CACHE_EXT: &str = "bin";

    #[derive(Debug, Deserialize, Serialize)]
    struct ChunkCache {
        local: IVec3,
        kind: chunk::ChunkKind,
    }

    #[cfg(test)]
    impl PartialEq for ChunkCache {
        fn eq(&self, other: &Self) -> bool {
            self.local == other.local && self.kind == other.kind
        }
    }

    pub(super) fn generate(local: IVec3) -> chunk::ChunkKind {
        let mut noise = FastNoise::seeded(15);
        noise.set_noise_type(NoiseType::SimplexFractal);
        noise.set_frequency(0.03);
        noise.set_fractal_type(FractalType::FBM);
        noise.set_fractal_octaves(3);
        noise.set_fractal_gain(0.9);
        noise.set_fractal_lacunarity(0.5);
        let world = chunk::to_world(local);
        let mut kind = chunk::ChunkKind::default();
        for x in 0..chunk::AXIS_SIZE {
            for z in 0..chunk::AXIS_SIZE {
                let h = noise.get_noise(world.x + x as f32, world.z + z as f32);
                let world_height = ((h + 1.0) / 2.0) * (2 * chunk::AXIS_SIZE) as f32;

                let height_local = world_height - world.y;

                if height_local < f32::EPSILON {
                    continue;
                }

                let end = usize::min(height_local as usize, chunk::AXIS_SIZE);

                for y in 0..end {
                    kind.set((x as i32, y as i32, z as i32).into(), 1.into());
                }
            }
        }
        let path = local_path(local);

        assert!(!path.exists(), "Cache already exists!");

        save(&path, local, &kind);

        kind
    }

    pub(super) fn save(path: &Path, local: IVec3, kind: &chunk::ChunkKind) {
        let cache = ChunkCache {
            local,
            kind: kind.clone(),
        };

        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)
            .unwrap_or_else(|_| panic!("Unable to write to file {}", path.display()));

        #[cfg(not(feature = "serde_ron"))]
        bincode::serialize_into(file, &cache)
            .unwrap_or_else(|_| panic!("Failed to serialize cache to file {}", path.display()));

        #[cfg(feature = "serde_ron")]
        ron::ser::to_writer(file, cache)
            .unwrap_or_else(|_| panic!("Failed to serialize cache to file {}", path.display()));
    }

    pub(super) fn load(path: &Path) -> chunk::ChunkKind {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(path)
            .unwrap_or_else(|_| panic!("Unable to open file {}", path.display()));

        #[cfg(not(feature = "serde_ron"))]
        let cache: ChunkCache = bincode::deserialize_from(file)
            .unwrap_or_else(|_| panic!("Failed to parse file {}", path.display()));

        #[cfg(feature = "serde_ron")]
        let cache =
            ron::de::from_reader(file).expect(&format!("Failed to parse file {}", path.display()));

        cache.kind
    }

    pub(super) fn local_path(local: IVec3) -> PathBuf {
        PathBuf::from(CACHE_PATH)
            .with_file_name(format_local(local))
            .with_extension(CACHE_EXT)
    }

    fn format_local(local: IVec3) -> String {
        local
            .to_string()
            .chars()
            .filter_map(|c| match c {
                ',' => Some('_'),
                ' ' | '[' | ']' => None,
                _ => Some(c),
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs::remove_file;

        #[test]
        #[should_panic]
        fn generate_cache_panic() {
            let local = (9999, 9998, 9997).into();
            let _ = remove_file(local_path(local));

            super::generate(local);
            super::generate(local);
        }

        #[test]
        fn local_path_test() {
            let path = super::local_path((0, 0, 0).into())
                .to_str()
                .unwrap()
                .to_string();

            assert!(path.ends_with(&format!("0_0_0.{}", CACHE_EXT)));

            let path = super::local_path((-1, 0, 0).into())
                .to_str()
                .unwrap()
                .to_string();

            assert!(path.ends_with(&format!("-1_0_0.{}", CACHE_EXT)));

            let path = super::local_path((-1, 3333, -461).into())
                .to_str()
                .unwrap()
                .to_string();

            assert!(path.ends_with(&format!("-1_3333_-461.{}", CACHE_EXT)));
        }

        #[test]
        fn test_ser_de() {
            let mut temp_file = std::env::temp_dir();
            temp_file.push("test.tmp");

            let cache = ChunkCache {
                local: IVec3::ZERO,
                kind: chunk::ChunkKind::default(),
            };

            create_cache(&temp_file, &cache);

            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(&temp_file)
                .unwrap();

            #[cfg(feature = "serde_ron")]
            let cache_loaded: ChunkCache = ron::de::from_reader(file).unwrap();

            #[cfg(not(feature = "serde_ron"))]
            let cache_loaded: ChunkCache = bincode::deserialize_from(file).unwrap();

            assert_eq!(cache, cache_loaded);
        }

        #[test]
        fn format_local() {
            assert_eq!("-234_22_1", super::format_local((-234, 22, 1).into()));
            assert_eq!(
                "-9999_-9999_-9999",
                super::format_local((-9999, -9999, -9999).into())
            );
            assert_eq!(
                "9999_-9999_9999",
                super::format_local((9999, -9999, 9999).into())
            );
            assert_eq!("0_0_0", super::format_local((0, 0, 0).into()));
        }

        fn get_test_path(local: IVec3) -> PathBuf {
            let working_path = env!("CARGO_WORKSPACE_DIR");
            let cache_path = local_path(local);

            Path::new(working_path).join(cache_path)
        }

        fn create_cache(path: &Path, cache: &ChunkCache) {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .unwrap();

            #[cfg(feature = "serde_ron")]
            ron::ser::to_writer(file, cache).unwrap();

            #[cfg(not(feature = "serde_ron"))]
            bincode::serialize_into(file, cache).unwrap();
        }

        #[test]
        fn load_cache() {
            let local = (-9998, 0, 9998).into();

            let cache = ChunkCache {
                local,
                kind: chunk::ChunkKind::default(),
            };

            let path = get_test_path(local);
            create_cache(&path, &cache);

            let loaded_kind = super::load(&path);

            assert_eq!(
                cache,
                ChunkCache {
                    local,
                    kind: loaded_kind,
                }
            );

            remove_file(path).unwrap();
        }

        #[test]
        fn save_cache() {
            let local = (-921, 0, 2319).into();

            let cache = ChunkCache {
                local,
                kind: chunk::ChunkKind::default(),
            };

            let path = get_test_path(local);

            assert!(!path.exists());

            super::save(&path, cache.local, &cache.kind);

            assert!(path.exists());

            let loaded_kind = super::load(&path);

            assert_eq!(
                cache,
                ChunkCache {
                    local,
                    kind: loaded_kind,
                }
            );

            remove_file(path).unwrap();
        }
    }
}
