use std::path::{Path, PathBuf};

use bevy::{prelude::*, utils::HashMap};
use bevy_retrograde::{
    core::image::{DynamicImage, GenericImageView},
    prelude::*,
};
use decorum::N32;
use itertools::Itertools;
use navmesh::NavMesh;

use crate::plugins::game::{
    assets::GameInfo,
    components::{
        DamageRegion, DamageRegionKnockBack, Enemy, Entrance, PhysicsGroup,
        TilesetTileCollisionMode, TilesetTileMetadata,
    },
};

/// Component that caches map tileset collision info
///
/// Keyed by (tileset_uid, tile_id)
pub struct LdtkMapTilesetTileCache(pub HashMap<(i32, i32), LdtkMapTilesetTileCacheItem>);
/// An item in the [`LdtkMapTilesetTileCache`]
#[derive(Clone)]
pub struct LdtkMapTilesetTileCacheItem {
    pub collision_shape: CollisionShape,
    pub damage_region: Option<DamageRegion>,
}
/// Component used to mark map collision shapes
pub struct LdtkMapTileCollisionShape;
/// Component used to mark the map as having had its collisions loaded
pub struct LdtkMapTileCollisionsLoaded;

/// Get any maps that have not had their tile collisions spawned yet and spawn them
pub fn spawn_map_collisions(
    mut commands: Commands,
    maps: Query<
        (Entity, &Handle<LdtkMap>, Option<&LdtkMapTilesetTileCache>),
        Without<LdtkMapTileCollisionsLoaded>,
    >,
    map_assets: Res<Assets<LdtkMap>>,
    image_assets: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
    game_info: Option<Res<GameInfo>>,
) {
    // Load game info or wait until it is loaded
    let game_info = if let Some(game_info) = game_info {
        game_info
    } else {
        return;
    };

    'map_load: for (map_ent, map_handle, tileset_tile_collisions_component) in maps.iter() {
        // Get map commands
        let mut map_commands = commands.entity(map_ent);

        // Get the loaded map
        let map = if let Some(map) = map_assets.get(map_handle) {
            map
        } else {
            continue;
        };

        // Load all tilesets and skip if any are missing
        let tileset_images = if let Some(tile_sets) = map
            .tile_sets
            .iter()
            .map(|(name, handle)| image_assets.get(handle).map(|image| (name, image)))
            .collect::<Option<HashMap<_, _>>>()
        {
            tile_sets
        } else {
            continue;
        };

        // Tilemap tile collisions indexed by (tileset_uid, tile_id)
        let mut tileset_tile_cache: HashMap<(i32, i32), LdtkMapTilesetTileCacheItem> =
            tileset_tile_collisions_component
                .map(|x| x.0.clone())
                .unwrap_or_default();

        // Generate collision shapes for all of the tiles in each tileset
        for tileset_def in &map.project.defs.tilesets {
            // For all tiles with custom data
            for tile_data in &tileset_def.custom_data {
                // Get tile ID and custom data
                let tile_id = tile_data
                    .get("tileId")
                    .expect("Tile data missing `tileId` field")
                    .as_i64()
                    .expect("Tile `tileId` field not an int") as i32;
                let data = tile_data
                    .get("data")
                    .expect("Tile data missing `data` field")
                    .as_str()
                    .expect("Tile `data` field not a string");

                // If we already have the collision calculated for this tile, skip it
                if tileset_tile_cache.contains_key(&(tileset_def.uid, tile_id)) {
                    continue;
                }

                // Parse tile metadata as YAML
                let tileset_tile_metadata: TilesetTileMetadata = match serde_yaml::from_str(data) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        warn!(
                            %error,
                            %tile_id,
                            tileset_id=%tileset_def.identifier,
                            "Could not parse tileset tile metadata, ignoring"
                        );
                        continue;
                    }
                };

                // Get the image for this tileset
                let tileset_image = *tileset_images
                    .get(&tileset_def.identifier)
                    .expect("Tileset image not loaded");

                // Helper for generating alpha-based collision shapes
                macro_rules! create_alpha_based_collision {
                    ($image:ident) => {
                        {
                            // Get the tile pixel x and y positions from the tile ID
                            let tile_grid_y = tile_id / tileset_def.__c_wid;
                            let tile_grid_x = tile_id - (tile_grid_y * tileset_def.__c_wid);
                            let tile_x = tile_grid_x * tileset_def.tile_grid_size;
                            let tile_y = tile_grid_y * tileset_def.tile_grid_size;

                            // Get the portion of the tilemap image for this tile
                            let tile_image = $image.view(
                                tile_x as u32,
                                tile_y as u32,
                                tileset_def.tile_grid_size as u32,
                                tileset_def.tile_grid_size as u32,
                            );

                            // Generate a collision shape from the tile image
                            let collision_shape = if let Some(collision) =
                                physics::create_convex_collider(
                                    DynamicImage::ImageRgba8(tile_image.to_image()),
                                    &TesselatedColliderConfig {
                                        vertice_separation: 1.,
                                        ..Default::default()
                                    },
                            ) {
                                collision
                            } else {
                                warn!(
                                    %tile_id,
                                    tileset_id=%tileset_def.identifier,
                                    "Could not create collision shape for tile"
                                );
                                continue;
                            };

                            collision_shape
                        }
                    }
                }

                // Get the tile collision shape
                let collision_shape = match tileset_tile_metadata.collision {
                    // Create a cuboid collision for this block
                    TilesetTileCollisionMode::Full => Some(CollisionShape::Cuboid {
                        half_extends: Vec3::new(
                            tileset_def.tile_grid_size as f32 / 2.0,
                            tileset_def.tile_grid_size as f32 / 2.0,
                            0.,
                        ),
                        border_radius: None,
                    }),
                    // Spawn a tesselated collision shape generated from
                    TilesetTileCollisionMode::FromAlpha => {
                        let collision_shape = create_alpha_based_collision!(tileset_image);

                        // Add the collision to the list
                        Some(collision_shape)
                    }
                    // Create a collision from the alpha of a corresponding tile in a reference tilesheet
                    TilesetTileCollisionMode::FromAlphaReference {
                        tileset: tileset_relative_path,
                    } => {
                        // Load the reference tileset image
                        let map_path = PathBuf::from(game_info.map.clone());
                        let tileset_reference_handle: Handle<Image> = asset_server.load_cached(
                            map_path
                                .parent()
                                .unwrap_or_else(|| Path::new("./"))
                                .join(tileset_relative_path),
                        );

                        // Get the reference tilesheet image
                        let tileset_reference_image = if let Some(tileset_image) =
                            image_assets.get(tileset_reference_handle)
                        {
                            tileset_image
                        // If the tilesheet image cannot be loaded
                        } else {
                            // Store the collisions we have currently and wait to try again next
                            // frame
                            map_commands.insert(LdtkMapTilesetTileCache(tileset_tile_cache));
                            continue 'map_load;
                        };

                        let collision_shape =
                            create_alpha_based_collision!(tileset_reference_image);

                        // Add the collision to the list
                        Some(collision_shape)
                    }
                    // Don't do anything for empty collisions
                    TilesetTileCollisionMode::None => None,
                };

                // If the tile has a collision shape, add it to the cache
                if let Some(collision_shape) = collision_shape {
                    tileset_tile_cache.insert(
                        (tileset_def.uid, tile_id),
                        LdtkMapTilesetTileCacheItem {
                            collision_shape,
                            damage_region: tileset_tile_metadata.damage_region.clone(),
                        },
                    );
                }
            }
        }

        // For every level in the map
        for level in &map.project.levels {
            // Get the level offset
            let level_offset = Vec3::new(level.world_x as f32, level.world_y as f32, 0.);

            // For every layer in the level
            for layer in level
                .layer_instances
                .as_ref()
                .expect("Map level has no layers")
                .iter()
            {
                // Get the layer offset
                let layer_offset = level_offset
                    + Vec3::new(
                        layer.__px_total_offset_x as f32,
                        layer.__px_total_offset_y as f32,
                        0.,
                    );

                // Get layer tile size
                let tile_size = layer.__grid_size as f32;

                // Get the NoCollision hlper layer for this layer if it exists
                let no_collision_layer = level
                    .layer_instances
                    .as_ref()
                    .expect("Level has no layers")
                    .iter()
                    .find(|x| x.__identifier == format!("{}NoCollision", layer.__identifier));

                // Get the layer tileset uid, or skip the layer if it doesn't have a tileset
                let tileset_uid = if let Some(uid) = layer.__tileset_def_uid {
                    uid
                } else {
                    continue;
                };

                // For every tile in the layer
                for tile in layer.grid_tiles.iter().chain(layer.auto_layer_tiles.iter()) {
                    // Skip this tile if it has a representative in the NoCollision layer
                    if let Some(no_collision_layer) = no_collision_layer {
                        let tile_index = (tile.px[0] / layer.__grid_size)
                            + (tile.px[1] / layer.__grid_size * layer.__c_wid);

                        // If the NoCollision layer has a tile in a position corresponding to this
                        // tile
                        if no_collision_layer.int_grid_csv[tile_index as usize] != 0 {
                            // Skip the tile
                            continue;
                        }
                    }

                    // Get the tile position
                    let tile_pos =
                        layer_offset + Vec3::new(tile.px[0] as f32, tile.px[1] as f32, 0.);

                    // Offset the tile position to get the center of the tile
                    let half_tile_size = Vec3::new(tile_size / 2.0, tile_size / 2.0, 0.);

                    // Spawn a collision shape for this tile if one exists
                    if let Some(tile_cache_item) = tileset_tile_cache.get(&(tileset_uid, tile.t)) {
                        map_commands.with_children(|map| {
                            // Spawn the entity with the collision shape
                            let mut entity_commands = map.spawn_bundle((
                                LdtkMapTileCollisionShape,
                                tile_cache_item.collision_shape.clone(),
                                CollisionLayers::from_bits(
                                    // Put it in the landscape group
                                    PhysicsGroup::Terrain.to_bits(),
                                    // And allow it to collide with all other layers
                                    PhysicsGroup::all_bits(),
                                ),
                                Transform::from_translation(tile_pos + half_tile_size),
                                GlobalTransform::default(),
                            ));

                            // If the tile has a damage region
                            if let Some(damage_region) = &tile_cache_item.damage_region {
                                // Add the damage region component as well
                                entity_commands.insert(damage_region.clone());
                            }
                        });
                    }
                }
            }
        }

        map_commands
            // Mark map collsions as loaded
            .insert(LdtkMapTileCollisionsLoaded)
            // Make the map a static body
            .insert(RigidBody::Static);
    }
}

pub fn hot_reload_map_collisions(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<LdtkMap>)>,
    tile_collisions: Query<(Entity, &Parent), With<LdtkMapTileCollisionShape>>,
    mut events: EventReader<AssetEvent<LdtkMap>>,
) {
    for event in events.iter() {
        if let AssetEvent::Modified { handle } = event {
            // For every map
            for (map_ent, map) in maps.iter() {
                // If this map's handle has been updated
                if map == handle {
                    // Unmark the map collisions as loaded and emove cached tileset collisions
                    commands
                        .entity(map_ent)
                        .remove::<LdtkMapTileCollisionsLoaded>()
                        .remove::<LdtkMapTilesetTileCache>();

                    // For every tile collision
                    for (tile_ent, parent) in tile_collisions.iter() {
                        // If this tile is a child of the map that changed
                        if parent.0 == map_ent {
                            // Despawn it
                            commands.entity(tile_ent).despawn();
                        }
                    }
                }
            }
        }
    }
}

/// A component containing the navigation meshes for all the levels in an LDtk map
pub struct LdtkMapLevelNavigationMeshes(pub HashMap<String, NavMesh>);
impl_deref!(LdtkMapLevelNavigationMeshes, HashMap<String, NavMesh>);

// Component for map navmesh debug visualization
pub struct LdtkMapLevelNavigationMeshDebugViz {
    pub level_id: String,
}

pub fn generate_map_navigation_mesh(
    mut commands: Commands,
    // All of the maps that have their tile collisions loaded, but do not have nav meshes
    maps: Query<
        (Entity, &Handle<LdtkMap>),
        (
            With<LdtkMapTileCollisionsLoaded>,
            With<LdtkMapEnemiesLoaded>,
            With<LdtkMapEntrancesLoaded>,
            Without<LdtkMapLevelNavigationMeshes>,
        ),
    >,
    map_assets: Res<Assets<LdtkMap>>,
    physics_world: bevy_retrograde::physics::heron::rapier_plugin::PhysicsWorld,
    game_info: Option<Res<GameInfo>>,
) {
    // For every map
    for (map_ent, map_handle) in maps.iter() {
        let map = if let Some(map) = map_assets.get(map_handle) {
            map
        } else {
            continue;
        };

        let mut meshes = HashMap::<String, NavMesh>::default();

        // For every level in the map
        for level in &map.project.levels {
            // Get the grid size from the first layer of this level
            let tile_size = level
                .layer_instances
                .as_ref()
                .expect("Level has no layers")
                .get(0)
                .expect("Level has no layers")
                .__grid_size as u32;

            // The size of the level in tiles
            let grid_size = UVec2::new(
                level.px_wid as u32 / tile_size,
                level.px_hei as u32 / tile_size,
            );

            // Get the level world offset
            let level_offset = Vec3::new(level.world_x as f32, level.world_y as f32, 0.);

            // Create a navigation mesh, using ray-casting to do edge testing
            let starting_point = level_offset.truncate() + Vec2::splat(tile_size as f32) / 2.;
            let edge_test = |v1: Vec2, v2: Vec2| {
                physics_world
                    .shape_cast_with_filter(
                        &CollisionShape::Sphere { radius: 4. },
                        v1.extend(0.),
                        Quat::default(),
                        (v2 - v1).extend(0.),
                        CollisionLayers::from_bits(
                            // In all groups
                            PhysicsGroup::all_bits(),
                            // Only collide with entrance shapes
                            PhysicsGroup::Terrain.to_bits(),
                        ),
                        |_| true,
                    )
                    .is_none()
            };

            // Create a triangulation point list
            let mut points =
                Vec::<delaunator::Point>::with_capacity((grid_size.x * grid_size.y) as usize);

            // For every node in the grid
            for x in 0..=grid_size.x {
                let x = x as f32;
                for y in 0..=grid_size.y {
                    let y = y as f32;

                    // Get the node coordinate
                    let pos = starting_point + Vec2::splat(tile_size as f32) * Vec2::new(x, y);

                    // And add it to the points list
                    points.push(delaunator::Point {
                        x: pos.x as f64,
                        y: pos.y as f64,
                    });
                }
            }

            // Triangulate the points
            let triangulation =
                delaunator::triangulate(&points).expect("Could not triangulate navigation mesh");

            let mut edge_test_results = HashMap::<[[N32; 2]; 2], bool>::default();

            // Convert triangles from Vec<usize> to Vec<[usize; 3]>
            let triangles = triangulation
                .triangles
                .iter()
                // .map(|&x| x as usize)
                .chunks(3)
                .into_iter()
                .map(|mut chunk| {
                    [
                        *chunk.next().unwrap(),
                        *chunk.next().unwrap(),
                        *chunk.next().unwrap(),
                    ]
                })
                // Discard any triangles where one of the edges doesn't pass the edge test
                .filter(|tri| {
                    let v1 = &points[tri[0]];
                    let v1 = [N32::from(v1.x as f32), N32::from(v1.y as f32)];

                    let v2 = &points[tri[1]];
                    let v2 = [N32::from(v2.x as f32), N32::from(v2.y as f32)];

                    let v3 = &points[tri[2]];
                    let v3 = [N32::from(v3.x as f32), N32::from(v3.y as f32)];

                    for edge in [[v1, v2], [v1, v3], [v2, v3]] {
                        let edge_reachable = *edge_test_results.entry(edge).or_insert_with(|| {
                            edge_test(
                                Vec2::new(edge[0][0].into(), edge[0][1].into()),
                                Vec2::new(edge[1][0].into(), edge[1][1].into()),
                            )
                        });
                        if !edge_reachable {
                            return false;
                        }
                    }

                    true
                })
                .collect::<Vec<_>>();

            // Convert our points to nav mesh vertices
            let vertices = points
                .into_iter()
                .map(|p| navmesh::NavVec3 {
                    x: p.x as f32,
                    y: p.y as f32,
                    z: 0.,
                })
                .collect::<Vec<_>>();
            // Convert our indices to navmesh indices
            let triangles = triangles
                .into_iter()
                .map(|t| navmesh::NavTriangle {
                    first: t[0] as u32,
                    second: t[1] as u32,
                    third: t[2] as u32,
                })
                .collect::<Vec<_>>();

            // Spawn debug visualization if enabled
            if game_info
                .as_ref()
                .map(|x| x.debug_rendering.navmesh)
                .unwrap_or_default()
            {
                commands
                    .spawn_bundle((
                        LdtkMapLevelNavigationMeshDebugViz {
                            level_id: level.identifier.clone(),
                        },
                        Transform::default(),
                        GlobalTransform::default(),
                    ))
                    .with_children(|viz| {
                        for vert in &vertices {
                            viz.spawn_bundle(ShapeBundle {
                                shape: Shape::circle_filled(
                                    epaint::pos2(vert.x as f32, vert.y as f32),
                                    1.,
                                    epaint::Color32::BLUE,
                                ),
                                transform: Transform::from_xyz(0., 0., 200.),
                                ..Default::default()
                            });
                        }

                        for triangle in &triangles {
                            let v1 = &vertices[triangle.first as usize];
                            let v2 = &vertices[triangle.second as usize];
                            let v3 = &vertices[triangle.third as usize];

                            viz.spawn_bundle(ShapeBundle {
                                shape: Shape::convex_polygon(
                                    vec![
                                        epaint::pos2(v1.x as f32, v1.y as f32),
                                        epaint::pos2(v2.x as f32, v2.y as f32),
                                        epaint::pos2(v3.x as f32, v3.y as f32),
                                    ],
                                    epaint::Color32::TRANSPARENT,
                                    (0.5, epaint::Color32::from_rgb(35, 18, 52)),
                                ),
                                transform: Transform::from_xyz(0., 0., 200.),
                                ..Default::default()
                            });
                        }
                    });
            }

            // Return the final navmesh
            let nav_mesh = NavMesh::new(vertices, triangles).expect("Could not create navmesh");

            meshes.insert(level.identifier.clone(), nav_mesh);
        }

        // Add the navigation meshes component to the map
        commands
            .entity(map_ent)
            .insert(LdtkMapLevelNavigationMeshes(meshes));
    }
}

pub struct LdtkMapEntrancesLoaded;

/// Spawn the entrance entities from the map
pub fn spawn_map_entrances(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<LdtkMap>), Without<LdtkMapEntrancesLoaded>>,
    map_assets: Res<Assets<LdtkMap>>,
) {
    // For every map
    for (ent, map_handle) in maps.iter() {
        // Get the map
        let map = if let Some(map) = map_assets.get(map_handle) {
            map
        } else {
            continue;
        };

        let mut map_commands = commands.entity(ent);

        // For every level in the map
        for level in &map.project.levels {
            // Get the level's position offest
            let level_offset = Vec3::new(level.world_x as f32, level.world_y as f32, 0.);

            // For every layer in the level
            for layer in level
                .layer_instances
                .as_ref()
                .expect("Map has no layers")
                .iter()
                .filter(|x| x.__type == "Entities")
            {
                // Get the layer offset
                let layer_offset = Vec3::new(
                    layer.__px_total_offset_x as f32,
                    layer.__px_total_offset_y as f32,
                    0.,
                );

                // Spawn collision sensors for the entrances
                for entrance in layer
                    .entity_instances
                    .iter()
                    .filter(|x| x.__identifier == "Entrance")
                {
                    let entrance_position = Vec3::new(
                        entrance.px[0] as f32 + layer.__grid_size as f32 / 2.,
                        entrance.px[1] as f32 + layer.__grid_size as f32 / 2.,
                        0.,
                    );

                    map_commands.with_children(|map| {
                        map.spawn_bundle((
                            Entrance {
                                map_handle: map_handle.clone(),
                                level: level.identifier.clone(),
                                id: entrance
                                    .field_instances
                                    .iter()
                                    .find(|x| x.__identifier == "id")
                                    .expect("Could not find entrance `id` field")
                                    .__value
                                    .as_str()
                                    .expect("Entrance `id` field is not a string")
                                    .into(),
                                to_level: entrance
                                    .field_instances
                                    .iter()
                                    .find(|x| x.__identifier == "to")
                                    .expect("Could not find entrance `to` field")
                                    .__value
                                    .as_str()
                                    .expect("Entrance `to` field is not a string")
                                    .into(),
                                spawn_at: entrance
                                    .field_instances
                                    .iter()
                                    .find(|x| x.__identifier == "spawn_at")
                                    .expect("Could not find entrance `spawn_at` field")
                                    .__value
                                    .as_str()
                                    .expect("Entrance `spawn_at` field is not a string")
                                    .into(),
                            },
                            CollisionShape::Cuboid {
                                half_extends: Vec3::new(
                                    // Shrink the entrance slightly by dividing by 2.2 to prevent
                                    // the collision from being hit past walls.
                                    entrance.width as f32 / 2.2,
                                    entrance.height as f32 / 2.2,
                                    0.,
                                ),
                                border_radius: None,
                            },
                            RigidBody::Sensor,
                            CollisionLayers::from_bits(
                                // In the entrance group
                                PhysicsGroup::Entrance.to_bits(),
                                // Can interact with all other groups
                                PhysicsGroup::all_bits(),
                            ),
                            Transform::from_translation(
                                level_offset + layer_offset + entrance_position,
                            ),
                            GlobalTransform::default(),
                        ));
                    });
                }
            }
        }

        map_commands.insert(LdtkMapEntrancesLoaded);
    }
}

pub fn hot_reload_map_entrances(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<LdtkMap>)>,
    entrances: Query<(Entity, &Entrance)>,
    mut events: EventReader<AssetEvent<LdtkMap>>,
) {
    for event in events.iter() {
        if let AssetEvent::Modified { handle } = event {
            // Remove the `LdtkMapEntrancesLoaded` flag from the map
            for (ent, map) in maps.iter() {
                if map == handle {
                    commands.entity(ent).remove::<LdtkMapEntrancesLoaded>();
                }
            }
            // Despawn all entrances for the modified map
            for (ent, entrance) in entrances.iter() {
                if &entrance.map_handle == handle {
                    commands.entity(ent).despawn();
                }
            }
        }
    }
}

pub struct LdtkMapEnemiesLoaded;
pub fn spawn_map_enemies(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<LdtkMap>), Without<LdtkMapEnemiesLoaded>>,
    map_assets: Res<Assets<LdtkMap>>,
    asset_server: Res<AssetServer>,
) {
    // For every map
    for (map_ent, map_handle) in maps.iter() {
        let map = if let Some(map) = map_assets.get(map_handle) {
            map
        } else {
            continue;
        };

        // For every level in the map
        for level in &map.project.levels {
            let level_offset = Vec3::new(level.world_x as f32, level.world_y as f32, 0.);

            // For every layer
            for layer in level.layer_instances.as_ref().expect("Level has no layers") {
                let layer_offset = level_offset
                    + Vec3::new(
                        layer.__px_total_offset_x as f32,
                        layer.__px_total_offset_y as f32,
                        0.,
                    );

                // For every enemy entity in the layer
                for entity in layer
                    .entity_instances
                    .iter()
                    .filter(|x| x.__identifier == "Enemy")
                {
                    let pos =
                        layer_offset + Vec3::new(entity.px[0] as f32, entity.px[1] as f32, 100.);

                    // Spawn an enemy
                    commands
                        .spawn_bundle(SpriteBundle {
                            image: asset_server.load("sprites/blueRadish.png"),
                            transform: Transform::from_translation(pos),
                            sprite: Sprite {
                                pixel_perfect: false,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(Enemy {
                            level: level.identifier.clone(),
                            map_handle: map_handle.clone(),
                        })
                        .insert(PhysicMaterial {
                            density: 100000.,
                            ..Default::default()
                        })
                        .insert(DamageRegion {
                            damage: 1,
                            knock_back: DamageRegionKnockBack {
                                speed: 800.,
                                force_duration: 0.04,
                                freeze_duration: 0.18,
                            },
                        })
                        .insert(RigidBody::Dynamic)
                        .insert(RotationConstraints::lock())
                        .insert(CollisionShape::Sphere { radius: 4. })
                        .insert(CollisionLayers::from_bits(
                            // In the enemy group
                            PhysicsGroup::Enemy.to_bits(),
                            // Can interact with all other groups
                            PhysicsGroup::all_bits(),
                        ))
                        .insert(Velocity::default());
                }
            }
        }

        // Mark map enemies as loaded
        commands.entity(map_ent).insert(LdtkMapEnemiesLoaded);
    }
}

pub fn hot_reload_map_enemies(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<LdtkMap>)>,
    enemies: Query<(Entity, &Enemy)>,
    mut events: EventReader<AssetEvent<LdtkMap>>,
) {
    for event in events.iter() {
        if let AssetEvent::Modified { handle } = event {
            // Remove the `LdtkMapEnemiesLoaded` flag from the map
            for (ent, map) in maps.iter() {
                if map == handle {
                    commands.entity(ent).remove::<LdtkMapEnemiesLoaded>();
                }
            }
            // Despawn all enemies for the modified map
            for (ent, enemy) in enemies.iter() {
                if &enemy.map_handle == handle {
                    commands.entity(ent).despawn();
                }
            }
        }
    }
}
