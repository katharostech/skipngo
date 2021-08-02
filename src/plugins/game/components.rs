use serde::Deserialize;

use bevy::{prelude::*, reflect::TypeUuid};
use bevy_retrograde::prelude::*;

//
// Game and level components
//

/// The current map level the player is in
#[derive(Clone)]
pub struct CurrentLevel(pub String);
impl_deref!(CurrentLevel, String);

#[derive(Clone)]
pub struct CurrentLevelMusic {
    pub sound_data: Handle<SoundData>,
    pub sound: Sound,
}

//
// Character components
//

#[derive(TypeUuid)]
#[uuid = "9fa5febb-1a7b-4864-9534-2d5df8df82f4"]
pub struct Character {
    pub name: String,
    pub max_health: u32,
    pub sprite_sheet_info: CharacterSpriteSheet,
    pub actions: CharacterActions,
    pub walk_speed: f32,
    pub sprite_image: Handle<Image>,
    pub sprite_sheet: Handle<SpriteSheet>,
    pub collision_shape: Handle<Image>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct CharacterYmlData {
    pub name: String,
    pub max_health: u32,
    pub sprite_sheet: CharacterSpriteSheet,
    pub actions: CharacterActions,
    pub walk_speed: f32,
    pub collision_shape: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CharacterSpriteSheet {
    pub path: String,
    pub grid_size: (u32, u32),
    pub tiles: (u32, u32),
}

#[derive(Deserialize)]
pub struct CharacterActions {
    pub walk: CharacterAction,
    pub idle: CharacterAction,
}

#[derive(Deserialize)]
pub struct CharacterAction {
    pub sound: Option<String>,
    pub animations: CharacterAnimations,
}

#[derive(Deserialize)]
pub struct CharacterAnimations {
    pub up: CharacterAnimation,
    pub down: CharacterAnimation,
    pub right: CharacterAnimation,
    pub left: CharacterAnimation,
}

#[derive(Deserialize)]
pub struct CharacterAnimation {
    #[serde(default)]
    pub flip: bool,
    pub frames: Vec<u32>,
}

#[derive(Clone)]
pub struct CharacterCurrentTilesetIndex(pub u32);

#[derive(Debug, Clone)]
pub enum CharacterStateAction {
    /// The player is walking
    Walk,
    /// The player is standing still
    Idle,
    /// The player is being knocked back from being damaged
    DamageKnockBack {
        /// The timer that will finish when the knock back is done being applied
        force_timer: Timer,
        /// The timer that will finish when the player controls should be restored
        freeze_timer: Timer,
    },
}

impl PartialEq for CharacterStateAction {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for CharacterStateAction {}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CharacterStateDirection {
    Up,
    Down,
    Left,
    Right,
}
pub struct CharacterState {
    pub action: CharacterStateAction,
    pub direction: CharacterStateDirection,
    pub anim_frame_idx: u32,
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            action: CharacterStateAction::Idle,
            direction: CharacterStateDirection::Down,
            anim_frame_idx: 0,
        }
    }
}

/// A bundle for spawning a character
// Copied mostly from the SpriteSheetBundle bundle
#[derive(Bundle, Default)]
pub struct CharacterBundle {
    pub character: Handle<Character>,
    pub state: CharacterState,
    #[bundle]
    pub sprite_bundle: SpriteBundle,
    pub sprite_sheet: Handle<SpriteSheet>,
}

//
// Physics
//

#[derive(heron::PhysicsLayer)]
pub enum PhysicsGroup {
    Terrain,
    Entrance,
    Player,
    Enemy,
}

//
// Map entities
//

/// An entrance on the map to another part of the map
#[derive(Debug, Clone)]
pub struct Entrance {
    /// A handle to the map that this entrance is for
    pub map_handle: Handle<LdtkMap>,
    /// A unique identifier for the entrance
    pub id: String,
    /// The level that this entrance is found in
    pub level: String,
    /// The map level that the entrance goes to
    pub to_level: String,
    /// The entrance in the `to` level that the entrance leads to
    pub spawn_at: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct TilesetTileMetadata {
    #[serde(default)]
    pub collision: TilesetTileCollisionMode,
    #[serde(default)]
    pub damage_region: Option<DamageRegion>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum TilesetTileCollisionMode {
    /// No collision for this tile
    None,
    /// Create a collision shape based on the tile alpha channel
    FromAlpha,
    /// Fill the whole tile square as the collision box
    Full,
    /// Create a collision based on the alpha of a tile in a tilesheet of the same size, that is
    /// used only for creating collision shapes
    FromAlphaReference {
        /// The path to the tilesheet to use as a collision reference
        tileset: String,
    },
}

impl Default for TilesetTileCollisionMode {
    fn default() -> Self {
        Self::None
    }
}

/// A damage region component that can be combined with a [`CollisionShape`] to hurt players or
/// other entities.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct DamageRegion {
    /// The amount of damage this tile should do when touching the player
    pub damage: u32,
    /// The knock back the damage region should apply
    pub knock_back: DamageRegionKnockBack,
}

/// The knockback attributes of a damage region
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct DamageRegionKnockBack {
    /// The speed at which to push the player when they touch the damage region
    pub speed: f32,
    /// How long to apply the knock-back force
    pub force_duration: f32,
    /// How long to freeze player controls
    pub freeze_duration: f32,
}

/// An enemy on the map
#[derive(Clone, Debug)]
pub struct Enemy {
    /// The identifier of the map level that the enemy is in
    pub level: String,
    /// The handle to the map this enemy is in
    pub map_handle: Handle<LdtkMap>,
}
