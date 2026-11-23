//! Entity and mob system matching LCE reference.

use crate::physics::{PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT};
use mclre_core::mth::{AABB, Vec3};
use mclre_core::{BlockStateId, GameRandom};
use mclre_world::{BlockPos, ChunkPos};
use std::collections::VecDeque;
use std::fmt::Debug;

/// Damage source for entity damage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageSource {
    Generic,
    EntityAttack(i32),
    Projectile(i32),
    Magic,
    Fire,
    Lava,
    Drowning,
    Suffocation,
    Fall,
    Cactus,
    Wither,
    Poison,
    Starvation,
    Thorns,
    Explosion,
    Anvil,
    Lightning,
    DragonBreath,
}

// ============================================================================
// Entity Type IDs (matching LCE reference)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum EntityType {
    // Living entities (mobs)
    Player = 0,
    Chicken = 1,
    Cow = 2,
    Pig = 3,
    Sheep = 4,
    Wolf = 5,
    Villager = 6,
    Mooshroom = 7,
    Squid = 8,
    Bat = 9,

    // Monsters
    Zombie = 10,
    Creeper = 11,
    Skeleton = 12,
    Spider = 13,
    ZombiePigman = 14,
    Slime = 15,
    Enderman = 16,
    Silverfish = 17,
    CaveSpider = 18,
    Ghast = 19,
    MagmaCube = 20,
    Blaze = 21,
    WitherSkeleton = 22,
    Witch = 23,

    // Boss
    EnderDragon = 24,
    Wither = 25,

    // Projectiles
    Arrow = 50,
    Snowball = 51,
    Fireball = 52,
    EnderPearl = 53,
    Egg = 54,
    EyeOfEnder = 55,
    Potion = 56,
    ExperienceBottle = 57,
    FireworkRocket = 58,

    // Vehicles
    Boat = 60,
    Minecart = 61,
    ChestMinecart = 62,
    FurnaceMinecart = 63,
    TNTMinecart = 64,
    HopperMinecart = 65,

    // Other
    ItemEntity = 70,
    ExperienceOrb = 71,
    LightningBolt = 72,
    FallingBlock = 73,
    ArmorStand = 74,
    EndCrystal = 75,
    LeashKnot = 76,
    Painting = 77,
    ItemFrame = 78,
}

impl EntityType {
    pub fn is_mob(self) -> bool {
        matches!(
            self,
            EntityType::Chicken
                | EntityType::Cow
                | EntityType::Pig
                | EntityType::Sheep
                | EntityType::Wolf
                | EntityType::Villager
                | EntityType::Mooshroom
                | EntityType::Squid
                | EntityType::Bat
                | EntityType::Zombie
                | EntityType::Creeper
                | EntityType::Skeleton
                | EntityType::Spider
                | EntityType::ZombiePigman
                | EntityType::Slime
                | EntityType::Enderman
                | EntityType::Silverfish
                | EntityType::CaveSpider
                | EntityType::Ghast
                | EntityType::MagmaCube
                | EntityType::Blaze
                | EntityType::WitherSkeleton
                | EntityType::Witch
                | EntityType::EnderDragon
                | EntityType::Wither
        )
    }

    pub fn is_monster(self) -> bool {
        matches!(
            self,
            EntityType::Zombie
                | EntityType::Creeper
                | EntityType::Skeleton
                | EntityType::Spider
                | EntityType::ZombiePigman
                | EntityType::Slime
                | EntityType::Enderman
                | EntityType::Silverfish
                | EntityType::CaveSpider
                | EntityType::Ghast
                | EntityType::MagmaCube
                | EntityType::Blaze
                | EntityType::WitherSkeleton
                | EntityType::Witch
                | EntityType::EnderDragon
                | EntityType::Wither
        )
    }

    pub fn is_animal(self) -> bool {
        matches!(
            self,
            EntityType::Chicken
                | EntityType::Cow
                | EntityType::Pig
                | EntityType::Sheep
                | EntityType::Wolf
                | EntityType::Mooshroom
                | EntityType::Squid
                | EntityType::Bat
        )
    }

    pub fn get_width(self) -> f32 {
        match self {
            EntityType::Player => PLAYER_HALF_WIDTH * 2.0,
            EntityType::Chicken => 0.4,
            EntityType::Cow => 0.9,
            EntityType::Pig => 0.9,
            EntityType::Sheep => 0.9,
            EntityType::Wolf => 0.6,
            EntityType::Villager => 0.6,
            EntityType::Zombie => 0.6,
            EntityType::Creeper => 0.6,
            EntityType::Skeleton => 0.6,
            EntityType::Spider => 1.4,
            EntityType::Enderman => 0.6,
            EntityType::Ghast => 4.0,
            EntityType::Slime => 0.6,
            EntityType::MagmaCube => 0.6,
            EntityType::Blaze => 0.6,
            EntityType::WitherSkeleton => 0.7,
            EntityType::Witch => 0.6,
            _ => 0.6,
        }
    }

    pub fn get_height(self) -> f32 {
        match self {
            EntityType::Player => PLAYER_HEIGHT,
            EntityType::Chicken => 0.7,
            EntityType::Cow => 1.4,
            EntityType::Pig => 0.9,
            EntityType::Sheep => 1.3,
            EntityType::Wolf => 0.85,
            EntityType::Villager => 1.95,
            EntityType::Zombie => 1.95,
            EntityType::Creeper => 1.7,
            EntityType::Skeleton => 1.99,
            EntityType::Spider => 0.9,
            EntityType::Enderman => 2.9,
            EntityType::Ghast => 4.0,
            EntityType::Slime => 0.6,
            EntityType::MagmaCube => 0.6,
            EntityType::Blaze => 1.8,
            EntityType::WitherSkeleton => 2.4,
            EntityType::Witch => 1.95,
            _ => 1.8,
        }
    }

    pub fn get_eye_height(self) -> f32 {
        match self {
            EntityType::Player => PLAYER_EYE_HEIGHT,
            EntityType::Chicken => 0.42,
            EntityType::Cow => 1.05,
            EntityType::Pig => 0.63,
            EntityType::Sheep => 0.91,
            EntityType::Wolf => 0.59,
            EntityType::Villager => 1.62,
            EntityType::Zombie => 1.62,
            EntityType::Creeper => 1.445,
            EntityType::Skeleton => 1.62,
            EntityType::Spider => 0.63,
            EntityType::Enderman => 2.55,
            EntityType::Ghast => 2.6,
            EntityType::Slime => 0.42,
            EntityType::MagmaCube => 0.42,
            EntityType::Blaze => 1.26,
            EntityType::WitherSkeleton => 2.04,
            EntityType::Witch => 1.62,
            _ => 1.62,
        }
    }
}

// ============================================================================
// Control Flags for AI goals
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ControlFlag {
    Move = 1 << 0,
    Look = 1 << 1,
    Jump = 1 << 2,
    Target = 1 << 3,
    UseItem = 1 << 4,
    Swim = 1 << 5,
    Fly = 1 << 6,
    Sneak = 1 << 7,
}

impl ControlFlag {
    pub const fn bits(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ControlFlags(u32);

impl ControlFlags {
    pub const NONE: Self = Self(0);
    pub const MOVE: Self = Self(1 << 0);
    pub const LOOK: Self = Self(1 << 1);
    pub const JUMP: Self = Self(1 << 2);
    pub const TARGET: Self = Self(1 << 3);
    pub const USE_ITEM: Self = Self(1 << 4);
    pub const SWIM: Self = Self(1 << 5);
    pub const FLY: Self = Self(1 << 6);
    pub const SNEAK: Self = Self(1 << 7);

    pub fn contains(self, flag: ControlFlag) -> bool {
        self.0 & flag.bits() != 0
    }

    pub fn insert(&mut self, flag: ControlFlag) {
        self.0 |= flag.bits();
    }

    pub fn remove(&mut self, flag: ControlFlag) {
        self.0 &= !flag.bits();
    }

    pub fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for ControlFlags {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl std::ops::BitOrAssign for ControlFlags {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl std::ops::BitAnd for ControlFlags {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

// ============================================================================
// Entity base class
// ============================================================================

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: i32,
    pub entity_type: EntityType,
    pub position: Vec3<f64>,
    pub velocity: Vec3<f64>,
    pub yaw: f32,
    pub pitch: f32,
    pub prev_yaw: f32,
    pub prev_pitch: f32,
    pub on_ground: bool,
    pub horizontal_collision: bool,
    pub vertical_collision: bool,
    pub collision: bool,
    pub removed: bool,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub bounding_box: AABB<f64>,
    pub no_clip: bool,
    pub fire_ticks: i16,
    pub max_fire_ticks: i16,
    pub in_water: bool,
    pub in_lava: bool,
    pub invulnerable_time: i16,
    pub age: i64,
    pub ticks_existed: i64,
    pub silent: bool,
    pub glowing: bool,
    pub glowing_color: i32,
    pub step_height: f32,
    pub pushable: bool,
    pub gravity: bool,
}

impl Entity {
    pub fn new(entity_type: EntityType, id: i32, position: Vec3<f64>) -> Self {
        let width = entity_type.get_width();
        let height = entity_type.get_height();
        let eye_height = entity_type.get_eye_height();

        let w = width as f64 / 2.0;
        let min = Vec3::new(position.x - w, position.y, position.z - w);
        let max = Vec3::new(position.x + w, position.y + height as f64, position.z + w);
        let bounding_box = AABB::new(min, max);

        Self {
            id,
            entity_type,
            position,
            velocity: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            prev_yaw: 0.0,
            prev_pitch: 0.0,
            on_ground: false,
            horizontal_collision: false,
            vertical_collision: false,
            collision: false,
            removed: false,
            width,
            height,
            eye_height,
            bounding_box,
            no_clip: false,
            fire_ticks: 0,
            max_fire_ticks: 15 * 20,
            in_water: false,
            in_lava: false,
            invulnerable_time: 0,
            age: 0,
            ticks_existed: 0,
            silent: false,
            glowing: false,
            glowing_color: 0,
            step_height: 0.6,
            pushable: true,
            gravity: true,
        }
    }

    pub fn get_bounding_box(&self) -> AABB<f64> {
        self.bounding_box
    }

    pub fn set_position(&mut self, pos: Vec3<f64>) {
        self.position = pos;
        let w = self.width as f64 / 2.0;
        let min = Vec3::new(pos.x - w, pos.y, pos.z - w);
        let max = Vec3::new(pos.x + w, pos.y + self.height as f64, pos.z + w);
        self.bounding_box = AABB::new(min, max);
    }

    pub fn move_relative(&mut self, xa: f64, za: f64, speed: f64) {
        let yaw_rad = (self.yaw as f64).to_radians();
        let xz = (xa * yaw_rad.cos() - za * yaw_rad.sin()) * speed;
        let zz = (xa * yaw_rad.sin() + za * yaw_rad.cos()) * speed;
        self.velocity.x += xz;
        self.velocity.z += zz;
    }

    pub fn is_alive(&self) -> bool {
        !self.removed
    }

    pub fn distance_to(&self, other: &Entity) -> f64 {
        let dx = self.position.x - other.position.x;
        let dy = self.position.y - other.position.y;
        let dz = self.position.z - other.position.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn tick(&mut self) {
        self.ticks_existed += 1;
        self.age += 1;

        self.prev_yaw = self.yaw;
        self.prev_pitch = self.pitch;

        if self.fire_ticks > 0 {
            self.fire_ticks -= 1;
        }

        if self.invulnerable_time > 0 {
            self.invulnerable_time -= 1;
        }
    }
}

// ============================================================================
// MoveControl - handles movement for mobs
// ============================================================================

#[derive(Debug)]
pub struct MoveControl {
    mob: i32, // entity ID reference
    speed: f32,
    jump: bool,
    target_x: f64,
    target_y: f64,
    target_z: f64,
    operation: MoveOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOperation {
    Wait,
    MoveTo,
    Strafe,
    Jump,
}

impl MoveControl {
    pub fn new(mob: i32) -> Self {
        Self {
            mob,
            speed: 0.0,
            jump: false,
            target_x: 0.0,
            target_y: 0.0,
            target_z: 0.0,
            operation: MoveOperation::Wait,
        }
    }

    pub fn set_wanted_position(&mut self, x: f64, y: f64, z: f64, speed: f32) {
        self.target_x = x;
        self.target_y = y;
        self.target_z = z;
        self.speed = speed;
        self.operation = MoveOperation::MoveTo;
    }

    pub fn set_jump(&mut self, jump: bool) {
        self.jump = jump;
    }

    pub fn tick(&mut self, entity: &mut Entity, level: &mut SimLevel) {
        match self.operation {
            MoveOperation::MoveTo => {
                let dx = self.target_x - entity.position.x;
                let dy = self.target_y - entity.position.y;
                let dz = self.target_z - entity.position.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();

                if dist < 0.25 {
                    self.operation = MoveOperation::Wait;
                    return;
                }

                let speed = self.speed as f64 / dist;
                entity.velocity.x += dx * speed * 0.1;
                entity.velocity.z += dz * speed * 0.1;
                entity.velocity.y += dy * speed * 0.1;

                // Update yaw to face target
                let yaw = (dx.atan2(dz)).to_degrees() as f32;
                entity.yaw = yaw - 90.0; // Adjust for Minecraft coordinate system
            }
            MoveOperation::Jump => {
                if entity.on_ground {
                    entity.velocity.y = crate::physics::JUMP_VELOCITY as f64;
                }
                self.operation = MoveOperation::Wait;
            }
            _ => {}
        }
    }
}

// ============================================================================
// JumpControl
// ============================================================================

#[derive(Debug)]
pub struct JumpControl {
    mob: i32,
    jumping: bool,
}

impl JumpControl {
    pub fn new(mob: i32) -> Self {
        Self {
            mob,
            jumping: false,
        }
    }

    pub fn set_jumping(&mut self, jumping: bool) {
        self.jumping = jumping;
    }

    pub fn tick(&mut self, entity: &mut Entity) {
        if self.jumping && entity.on_ground {
            entity.velocity.y = crate::physics::JUMP_VELOCITY as f64;
            self.jumping = false;
        }
    }
}

// ============================================================================
// Goal trait and GoalSelector
// ============================================================================

pub trait Goal: Debug + Send + Sync {
    fn can_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool;
    fn can_continue_to_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool;
    fn start(&mut self, entity: &mut MobEntity, level: &mut SimLevel);
    fn stop(&mut self, entity: &mut MobEntity, level: &mut SimLevel);
    fn tick(&mut self, entity: &mut MobEntity, level: &mut SimLevel);
    fn get_priority(&self) -> i32;
    fn get_required_control_flags(&self) -> ControlFlags;
    fn can_interrupt(&self) -> bool {
        true
    }
    fn get_name(&self) -> &'static str {
        "Goal"
    }
}

#[derive(Debug)]
struct InternalGoal {
    goal: Box<dyn Goal>,
    priority: i32,
    is_using: bool,
}

#[derive(Debug, Default)]
pub struct GoalSelector {
    goals: Vec<InternalGoal>,
}

impl GoalSelector {
    pub fn new() -> Self {
        Self { goals: Vec::new() }
    }

    pub fn add_goal(&mut self, priority: i32, goal: Box<dyn Goal>) {
        self.goals.push(InternalGoal {
            goal,
            priority,
            is_using: false,
        });
        // Sort by priority (lower = higher priority)
        self.goals.sort_by_key(|g| g.priority);
    }

    pub fn tick(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        let mut to_start = Vec::new();
        let mut to_stop = Vec::new();

        // First pass: check running goals and collect what needs to stop/start
        // We need to collect the data first to avoid borrow conflicts
        let goals_snapshot: Vec<(i32, bool)> = self
            .goals
            .iter()
            .map(|ig| (ig.priority, ig.is_using))
            .collect();

        for (priority, is_using) in goals_snapshot {
            // Find the goal again to check conditions
            let ig_opt = self.goals.iter().find(|g| g.priority == priority);
            if ig_opt.is_none() {
                continue;
            }
            let ig = ig_opt.unwrap();

            if is_using {
                // Need to check can_use_in_system - we'll do this in a separate pass
                let can_continue = self.can_use_in_system_for_priority(priority, entity, level)
                    && ig.goal.can_continue_to_use(entity, level);
                if !can_continue {
                    to_stop.push(priority);
                }
            } else {
                let can_use = self.can_use_in_system_for_priority(priority, entity, level)
                    && ig.goal.can_use(entity, level);
                if can_use {
                    to_start.push(priority);
                    // Mark as using
                    if let Some(ig_mut) = self.goals.iter_mut().find(|g| g.priority == priority) {
                        ig_mut.is_using = true;
                    }
                }
            }
        }

        // Stop goals that can no longer run
        for priority in to_stop {
            if let Some(ig) = self.goals.iter_mut().find(|g| g.priority == priority) {
                ig.goal.stop(entity, level);
                ig.is_using = false;
            }
        }

        // Start new goals
        for priority in to_start {
            if let Some(ig) = self
                .goals
                .iter_mut()
                .find(|g| g.priority == priority && g.is_using)
            {
                ig.goal.start(entity, level);
            }
        }

        // Tick running goals
        for ig in &mut self.goals {
            if ig.is_using {
                ig.goal.tick(entity, level);
            }
        }
    }

    fn can_use_in_system_for_priority(
        &self,
        priority: i32,
        entity: &MobEntity,
        level: &mut SimLevel,
    ) -> bool {
        // Find the goal with this priority
        let ig_opt = self.goals.iter().find(|g| g.priority == priority);
        if ig_opt.is_none() {
            return false;
        }
        let ig = ig_opt.unwrap();
        self.can_use_in_system(ig, entity, level)
    }

    fn can_use_in_system(
        &self,
        ig: &InternalGoal,
        entity: &MobEntity,
        level: &mut SimLevel,
    ) -> bool {
        for other in &self.goals {
            if std::ptr::eq(ig.goal.as_ref(), other.goal.as_ref()) && ig.priority == other.priority
            {
                continue;
            }
            if ig.priority >= other.priority {
                if other.is_using && !self.can_coexist(ig, other) {
                    return false;
                }
            } else if other.is_using && !other.goal.can_interrupt() {
                return false;
            }
        }
        true
    }

    fn can_coexist(&self, a: &InternalGoal, b: &InternalGoal) -> bool {
        (a.goal.get_required_control_flags() & b.goal.get_required_control_flags()).is_empty()
    }
}

// ============================================================================
// Concrete Goal Implementations
// ============================================================================

/// PanicGoal - makes animals run away when hurt
#[derive(Debug)]
pub struct PanicGoal {
    speed_modifier: f32,
}

impl PanicGoal {
    pub fn new(speed_modifier: f32) -> Self {
        Self { speed_modifier }
    }
}

impl Goal for PanicGoal {
    fn can_use(&self, entity: &MobEntity, _level: &mut SimLevel) -> bool {
        entity.last_hurt_time > 0 && entity.health > 0.0
    }

    fn can_continue_to_use(&self, entity: &MobEntity, _level: &mut SimLevel) -> bool {
        entity.last_hurt_time > 0 && entity.health > 0.0
    }

    fn start(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.move_control.operation = MoveOperation::MoveTo;
        entity.speed *= self.speed_modifier;
    }

    fn stop(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed /= self.speed_modifier;
    }

    fn tick(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        // Find a safe position away from the threat
        // Simplified: just move in a random direction
        let mut random = level.rng();
        let angle = random.next_float() * 2.0 * std::f32::consts::PI;
        let dist = 10.0 + random.next_float() * 10.0;
        let x = entity.entity.position.x + angle.cos() as f64 * dist as f64;
        let z = entity.entity.position.z + angle.sin() as f64 * dist as f64;
        let y = entity.entity.position.y;
        entity
            .move_control
            .set_wanted_position(x, y, z, entity.speed);
    }

    fn get_priority(&self) -> i32 {
        1
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::MOVE
    }
    fn get_name(&self) -> &'static str {
        "PanicGoal"
    }
}

/// BreedGoal - makes animals breed when in love mode
#[derive(Debug)]
pub struct BreedGoal {
    speed_modifier: f32,
}

impl BreedGoal {
    pub fn new(speed_modifier: f32) -> Self {
        Self { speed_modifier }
    }
}

impl Goal for BreedGoal {
    fn can_use(&self, entity: &MobEntity, _level: &mut SimLevel) -> bool {
        entity.is_baby == false && entity.age > 6000 // adult and not recently bred
    }

    fn can_continue_to_use(&self, entity: &MobEntity, _level: &mut SimLevel) -> bool {
        self.can_use(entity, _level)
    }

    fn start(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed *= self.speed_modifier;
    }

    fn stop(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed /= self.speed_modifier;
    }

    fn tick(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        // Look for a partner nearby
        // Simplified: just move randomly
        let mut random = level.rng();
        let angle = random.next_float() * 2.0 * std::f32::consts::PI;
        let dist = 5.0 + random.next_float() * 5.0;
        let x = entity.entity.position.x + angle.cos() as f64 * dist as f64;
        let z = entity.entity.position.z + angle.sin() as f64 * dist as f64;
        let y = entity.entity.position.y;
        entity
            .move_control
            .set_wanted_position(x, y, z, entity.speed);
    }

    fn get_priority(&self) -> i32 {
        2
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::MOVE
    }
    fn get_name(&self) -> &'static str {
        "BreedGoal"
    }
}

/// FloatGoal - makes entities swim
#[derive(Debug)]
pub struct FloatGoal;

impl FloatGoal {
    pub fn new() -> Self {
        Self
    }
}

impl Goal for FloatGoal {
    fn can_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool {
        entity.entity.in_water && !entity.entity.on_ground
    }

    fn can_continue_to_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool {
        self.can_use(entity, level)
    }

    fn start(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.jump_control.set_jumping(true);
    }

    fn stop(&mut self, _entity: &mut MobEntity, _level: &mut SimLevel) {}

    fn tick(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        // Swim upward
        if entity.entity.velocity.y < 0.0 {
            entity.entity.velocity.y = 0.04;
        }
        entity.entity.velocity.y += 0.005;
    }

    fn get_priority(&self) -> i32 {
        0
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::MOVE | ControlFlags::JUMP
    }
    fn get_name(&self) -> &'static str {
        "FloatGoal"
    }
}

/// RandomStrollGoal - makes entities wander randomly
#[derive(Debug)]
pub struct RandomStrollGoal {
    speed_modifier: f32,
    chance: f32,
    last_stroll: i32,
}

impl RandomStrollGoal {
    pub fn new(speed_modifier: f32, chance: f32) -> Self {
        Self {
            speed_modifier,
            chance,
            last_stroll: 0,
        }
    }
}

impl Goal for RandomStrollGoal {
    fn can_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool {
        if entity.move_control.operation != MoveOperation::Wait {
            return false;
        }
        let mut random = level.rng();
        random.next_float() < self.chance
    }

    fn can_continue_to_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool {
        entity.move_control.operation == MoveOperation::MoveTo
    }

    fn start(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        let mut random = level.rng();
        let angle = random.next_float() * 2.0 * std::f32::consts::PI;
        let dist = 5.0 + random.next_float() * 15.0;
        let x = entity.entity.position.x + angle.cos() as f64 * dist as f64;
        let z = entity.entity.position.z + angle.sin() as f64 * dist as f64;
        let y = entity.entity.position.y;
        entity
            .move_control
            .set_wanted_position(x, y, z, entity.speed * self.speed_modifier);
        entity.speed *= self.speed_modifier;
        self.last_stroll = 0;
    }

    fn stop(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed /= self.speed_modifier;
        entity.move_control.operation = MoveOperation::Wait;
    }

    fn tick(&mut self, _entity: &mut MobEntity, _level: &mut SimLevel) {
        self.last_stroll += 1;
        if self.last_stroll > 200 { // 10 seconds timeout
            // Will be stopped by can_continue_to_use
        }
    }

    fn get_priority(&self) -> i32 {
        5
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::MOVE
    }
    fn get_name(&self) -> &'static str {
        "RandomStrollGoal"
    }
}

/// LookAtPlayerGoal - makes entities look at nearby players
#[derive(Debug)]
pub struct LookAtPlayerGoal {
    look_distance: f64,
    look_time: i32,
}

impl LookAtPlayerGoal {
    pub fn new(look_distance: f64) -> Self {
        Self {
            look_distance,
            look_time: 0,
        }
    }
}

impl Goal for LookAtPlayerGoal {
    fn can_use(&self, _entity: &MobEntity, _level: &mut SimLevel) -> bool {
        // Simplified: always can use if there are players nearby
        true
    }

    fn can_continue_to_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool {
        self.can_use(entity, level)
    }

    fn start(&mut self, _entity: &mut MobEntity, level: &mut SimLevel) {
        self.look_time = 40 + level.rng().next_int_bound(20);
    }

    fn stop(&mut self, _entity: &mut MobEntity, _level: &mut SimLevel) {}

    fn tick(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        self.look_time -= 1;
        // Look at a random nearby position (simulating looking at player)
        let mut random = level.rng();
        let x = entity.entity.position.x + (random.next_float() - 0.5) as f64 * 10.0;
        let y = entity.entity.position.y + entity.entity.eye_height as f64;
        let z = entity.entity.position.z + (random.next_float() - 0.5) as f64 * 10.0;
        entity.look_at(x, y, z);
    }

    fn get_priority(&self) -> i32 {
        6
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::LOOK
    }
    fn get_name(&self) -> &'static str {
        "LookAtPlayerGoal"
    }
}

/// RandomLookAroundGoal - makes entities look around randomly
#[derive(Debug)]
pub struct RandomLookAroundGoal;

impl RandomLookAroundGoal {
    pub fn new() -> Self {
        Self
    }
}

impl Goal for RandomLookAroundGoal {
    fn can_use(&self, _entity: &MobEntity, _level: &mut SimLevel) -> bool {
        true
    }

    fn can_continue_to_use(&self, _entity: &MobEntity, _level: &mut SimLevel) -> bool {
        true
    }

    fn start(&mut self, _entity: &mut MobEntity, _level: &mut SimLevel) {}

    fn stop(&mut self, _entity: &mut MobEntity, _level: &mut SimLevel) {}

    fn tick(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        if level.rng().next_int_bound(10) == 0 {
            let mut random = level.rng();
            let yaw = (random.next_float() - 0.5) * 180.0;
            let pitch = (random.next_float() - 0.5) * 90.0;
            entity.entity.yaw += yaw;
            entity.entity.pitch = (entity.entity.pitch + pitch).clamp(-90.0, 90.0);
        }
    }

    fn get_priority(&self) -> i32 {
        7
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::LOOK
    }
    fn get_name(&self) -> &'static str {
        "RandomLookAroundGoal"
    }
}

/// MeleeAttackGoal - makes hostile mobs attack players
#[derive(Debug)]
pub struct MeleeAttackGoal {
    speed_modifier: f32,
    attack_cooldown: i32,
}

impl MeleeAttackGoal {
    pub fn new(speed_modifier: f32) -> Self {
        Self {
            speed_modifier,
            attack_cooldown: 0,
        }
    }
}

impl Goal for MeleeAttackGoal {
    fn can_use(&self, entity: &MobEntity, _level: &mut SimLevel) -> bool {
        entity.attack_target.is_some()
    }

    fn can_continue_to_use(&self, entity: &MobEntity, _level: &mut SimLevel) -> bool {
        entity.attack_target.is_some() && entity.attack_time == 0
    }

    fn start(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed *= self.speed_modifier;
    }

    fn stop(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed /= self.speed_modifier;
        entity.attack_target = None;
    }

    fn tick(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        if let Some(target_id) = entity.attack_target {
            // Move towards target
            if let Some(target) = level.get_entity(target_id) {
                entity.move_control.set_wanted_position(
                    target.entity.position.x,
                    target.entity.position.y,
                    target.entity.position.z,
                    entity.speed * self.speed_modifier,
                );

                // Attack if close enough
                let dist = entity.entity.distance_to(&target.entity);
                if dist < 2.0 && entity.attack_time == 0 {
                    entity.attack_time = 10; // cooldown
                    entity.swing_time = 8;
                    // Deal damage - need mutable access
                    if let Some(target_mut) = level.get_entity_mut(target_id) {
                        target_mut.hurt(DamageSource::EntityAttack(entity.entity.id), 2.0);
                    }
                }
            }
        }

        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }
    }

    fn get_priority(&self) -> i32 {
        3
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::MOVE | ControlFlags::TARGET
    }
    fn get_name(&self) -> &'static str {
        "MeleeAttackGoal"
    }
}

/// WaterAvoidingRandomStrollGoal - like RandomStroll but avoids water
#[derive(Debug)]
pub struct WaterAvoidingRandomStrollGoal {
    speed_modifier: f32,
    chance: f32,
    last_stroll: i32,
}

impl WaterAvoidingRandomStrollGoal {
    pub fn new(speed_modifier: f32, chance: f32) -> Self {
        Self {
            speed_modifier,
            chance,
            last_stroll: 0,
        }
    }
}

impl Goal for WaterAvoidingRandomStrollGoal {
    fn can_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool {
        if entity.move_control.operation != MoveOperation::Wait {
            return false;
        }
        if entity.entity.in_water {
            return false;
        }
        level.rng().next_float() < self.chance
    }

    fn can_continue_to_use(&self, entity: &MobEntity, level: &mut SimLevel) -> bool {
        entity.move_control.operation == MoveOperation::MoveTo && !entity.entity.in_water
    }

    fn start(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        // Generate all random values upfront to avoid borrow conflicts
        let angle = level.rng().next_float() * 2.0 * std::f32::consts::PI;
        let dist = 5.0 + level.rng().next_float() * 15.0;
        let x = entity.entity.position.x + angle.cos() as f64 * dist as f64;
        let z = entity.entity.position.z + angle.sin() as f64 * dist as f64;
        let y = entity.entity.position.y;

        // Check if position is not in water (simplified)
        let block_pos = mclre_world::BlockPos::new(x as i32, y as i32, z as i32);
        if let Some(block_state) = level.get_block(block_pos) {
            if block_state != mclre_core::BlockStateId(8)
                && block_state != mclre_core::BlockStateId(9)
            {
                entity.move_control.set_wanted_position(
                    x,
                    y,
                    z,
                    entity.speed * self.speed_modifier,
                );
                entity.speed *= self.speed_modifier;
                self.last_stroll = 0;
                return;
            }
        }
    }

    fn stop(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed /= self.speed_modifier;
        entity.move_control.operation = MoveOperation::Wait;
    }

    fn tick(&mut self, _entity: &mut MobEntity, _level: &mut SimLevel) {
        self.last_stroll += 1;
        if self.last_stroll > 200 {
            // timeout
        }
    }

    fn get_priority(&self) -> i32 {
        4
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::MOVE
    }
    fn get_name(&self) -> &'static str {
        "WaterAvoidingRandomStrollGoal"
    }
}

/// FollowParentGoal - baby animals follow their parents
#[derive(Debug)]
pub struct FollowParentGoal {
    speed_modifier: f32,
}

impl FollowParentGoal {
    pub fn new(speed_modifier: f32) -> Self {
        Self { speed_modifier }
    }
}

impl Goal for FollowParentGoal {
    fn can_use(&self, entity: &MobEntity, _level: &mut SimLevel) -> bool {
        entity.is_baby
    }

    fn can_continue_to_use(&self, entity: &MobEntity, _level: &mut SimLevel) -> bool {
        entity.is_baby
    }

    fn start(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed *= self.speed_modifier;
    }

    fn stop(&mut self, entity: &mut MobEntity, _level: &mut SimLevel) {
        entity.speed /= self.speed_modifier;
    }

    fn tick(&mut self, entity: &mut MobEntity, level: &mut SimLevel) {
        // Find nearest adult of same type
        let mut closest = None;
        let mut closest_dist = f64::MAX;

        for other in &level.entities {
            if other.entity.entity_type == entity.entity.entity_type
                && !other.is_baby
                && other.entity.id != entity.entity.id
            {
                let dist = entity.entity.distance_to(&other.entity);
                if dist < closest_dist && dist < 16.0 {
                    closest_dist = dist;
                    closest = Some(other.entity.id);
                }
            }
        }

        if let Some(parent_id) = closest {
            if let Some(parent) = level.get_entity(parent_id) {
                entity.move_control.set_wanted_position(
                    parent.entity.position.x,
                    parent.entity.position.y,
                    parent.entity.position.z,
                    entity.speed * self.speed_modifier,
                );
            }
        }
    }

    fn get_priority(&self) -> i32 {
        3
    }
    fn get_required_control_flags(&self) -> ControlFlags {
        ControlFlags::MOVE
    }
    fn get_name(&self) -> &'static str {
        "FollowParentGoal"
    }
}

// ============================================================================
// Path Navigation (A* pathfinding) - matching LCE reference
// ============================================================================

/// Node type for pathfinding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathNodeType {
    Open = 0,
    Walkable = 1,
    Water = 2,
    Lava = 3,
    Fence = 4,
    Trap = 5,
    Closed = 6,
}

/// A* pathfinding node
#[derive(Debug, Clone)]
pub struct PathNode {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub g: f32, // cost from start
    pub h: f32, // heuristic to target
    pub f: f32, // total cost (g + h)
    pub closed: bool,
    pub in_open_set: bool,
    pub came_from: Option<Box<PathNode>>,
}

impl PathNode {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            x,
            y,
            z,
            g: 0.0,
            h: 0.0,
            f: 0.0,
            closed: false,
            in_open_set: false,
            came_from: None,
        }
    }

    pub fn distance_to_sqr(&self, other: &PathNode) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        let dz = (self.z - other.z) as f32;
        dx * dx + dy * dy + dz * dz
    }

    pub fn equals(&self, other: &PathNode) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }

    pub fn hash(&self) -> i32 {
        // Same hash as LCE: x * 31 + y * 31 * 31 + z * 31 * 31 * 31
        self.x * 31 + self.y * 31 * 31 + self.z * 31 * 31 * 31
    }
}

/// Result path from pathfinding
#[derive(Debug, Clone)]
pub struct Path {
    pub nodes: Vec<PathNode>,
    pub current_index: usize,
}

impl Path {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            current_index: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_next_node(&mut self) -> Option<&PathNode> {
        if self.current_index < self.nodes.len() {
            let node = &self.nodes[self.current_index];
            self.current_index += 1;
            Some(node)
        } else {
            None
        }
    }

    pub fn get_node(&self, index: usize) -> Option<&PathNode> {
        self.nodes.get(index)
    }

    pub fn last_node(&self) -> Option<&PathNode> {
        self.nodes.last()
    }
}

/// Pathfinding result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathResult {
    Success,
    NoPath,
    DestinationBlocked,
}

/// Binary heap for A* open set (min-heap by f score)
#[derive(Debug, Default)]
pub struct BinaryHeap {
    data: Vec<PathNode>,
}

impl BinaryHeap {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn push(&mut self, node: PathNode) {
        self.data.push(node);
        self.sift_up(self.data.len() - 1);
    }

    pub fn pop(&mut self) -> Option<PathNode> {
        if self.data.is_empty() {
            return None;
        }
        let min = self.data.swap_remove(0);
        if !self.data.is_empty() {
            self.sift_down(0);
        }
        Some(min)
    }

    pub fn peek(&self) -> Option<&PathNode> {
        self.data.first()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    fn sift_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.data[index].f >= self.data[parent].f {
                break;
            }
            self.data.swap(index, parent);
            index = parent;
        }
    }

    fn sift_down(&mut self, mut index: usize) {
        loop {
            let left = 2 * index + 1;
            let right = 2 * index + 2;
            let mut smallest = index;

            if left < self.data.len() && self.data[left].f < self.data[smallest].f {
                smallest = left;
            }
            if right < self.data.len() && self.data[right].f < self.data[smallest].f {
                smallest = right;
            }
            if smallest == index {
                break;
            }
            self.data.swap(index, smallest);
            index = smallest;
        }
    }

    pub fn change_cost(&mut self, index: usize, new_f: f32) {
        if index >= self.data.len() {
            return;
        }
        let old_f = self.data[index].f;
        self.data[index].f = new_f;
        if new_f < old_f {
            self.sift_up(index);
        } else if new_f > old_f {
            self.sift_down(index);
        }
    }

    pub fn find_index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        self.data
            .iter()
            .position(|n| n.x == x && n.y == y && n.z == z)
    }
}

/// Pathfinder using A* algorithm (matching LCE PathFinder)
#[derive(Debug)]
pub struct PathFinder {
    pub open_set: BinaryHeap,
    pub nodes: std::collections::HashMap<i32, PathNode>,
    pub neighbors: Vec<PathNode>,
    pub can_pass_doors: bool,
    pub can_open_doors: bool,
    pub avoid_water: bool,
    pub can_float: bool,
}

impl PathFinder {
    pub fn new(
        can_pass_doors: bool,
        can_open_doors: bool,
        avoid_water: bool,
        can_float: bool,
    ) -> Self {
        Self {
            open_set: BinaryHeap::new(),
            nodes: std::collections::HashMap::new(),
            neighbors: Vec::with_capacity(32),
            can_pass_doors,
            can_open_doors,
            avoid_water,
            can_float,
        }
    }

    /// Find path from entity to target position
    pub fn find_path_to_pos(
        &mut self,
        level: &SimLevel,
        entity: &MobEntity,
        target_x: f64,
        target_y: f64,
        target_z: f64,
        max_dist: f32,
    ) -> Option<Path> {
        self.open_set = BinaryHeap::new();
        self.nodes.clear();
        self.neighbors.clear();

        let entity_width = entity.entity.width;
        let entity_height = entity.entity.height;

        // Calculate start position (entity feet)
        let start_x = entity.entity.position.x.floor() as i32;
        let start_y = (entity.entity.position.y + 0.5).floor() as i32;
        let start_z = entity.entity.position.z.floor() as i32;

        // Calculate target node
        let target_x = (target_x - entity_width as f64 / 2.0).floor() as i32;
        let target_y = target_y.floor() as i32;
        let target_z = (target_z - entity_width as f64 / 2.0).floor() as i32;

        // Create start and target nodes
        let from = self.get_or_create_node(start_x, start_y, start_z);
        let to = self.get_or_create_node(target_x, target_y, target_z);

        // Size of entity for collision checking
        let size_x = (entity.entity.width + 1.0).floor() as i32;
        let size_y = (entity.entity.height + 1.0).floor() as i32;
        let size_z = (entity.entity.width + 1.0).floor() as i32;

        // Find path
        self.find_path(level, entity, &from, &to, size_x, size_y, size_z, max_dist)
    }

    fn get_or_create_node(&mut self, x: i32, y: i32, z: i32) -> PathNode {
        let hash = Self::hash_coords(x, y, z);
        if let Some(node) = self.nodes.get(&hash) {
            node.clone()
        } else {
            let node = PathNode::new(x, y, z);
            self.nodes.insert(hash, node.clone());
            node
        }
    }

    fn hash_coords(x: i32, y: i32, z: i32) -> i32 {
        x * 31 + y * 31 * 31 + z * 31 * 31 * 31
    }

    fn find_path(
        &mut self,
        level: &SimLevel,
        entity: &MobEntity,
        from: &PathNode,
        to: &PathNode,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        max_dist: f32,
    ) -> Option<Path> {
        // Initialize start node
        let from_hash = Self::hash_coords(from.x, from.y, from.z);
        if let Some(node) = self.nodes.get_mut(&from_hash) {
            node.g = 0.0;
            node.h = from.distance_to_sqr(to) as f32;
            node.f = node.h;
        }
        let mut from_node = self
            .nodes
            .get(&Self::hash_coords(from.x, from.y, from.z))
            .cloned()
            .unwrap_or_else(|| from.clone());

        self.open_set = BinaryHeap::new();
        self.open_set.push(from_node.clone());
        if let Some(node) = self
            .nodes
            .get_mut(&Self::hash_coords(from.x, from.y, from.z))
        {
            node.in_open_set = true;
        }

        let mut closest = from_node.clone();

        while !self.open_set.is_empty() {
            let current = self.open_set.pop().unwrap();

            if current.equals(&to) {
                return Some(self.reconstruct_path(&current));
            }

            if current.distance_to_sqr(&to) < closest.distance_to_sqr(&to) {
                closest = current.clone();
            }

            // Mark as closed
            let current_hash = Self::hash_coords(current.x, current.y, current.z);
            if let Some(node) = self.nodes.get_mut(&current_hash) {
                node.closed = true;
            }

            // Get neighbors
            let neighbor_count = self.get_neighbors(level, &current, size_x, size_y, size_z, &to);
            for i in 0..neighbor_count {
                let neighbor = self.neighbors[i].clone();
                let neighbor_hash = Self::hash_coords(neighbor.x, neighbor.y, neighbor.z);

                if self.nodes.get(&neighbor_hash).map_or(false, |n| n.closed) {
                    continue;
                }

                let tentative_g = current.g + current.distance_to_sqr(&neighbor) as f32;

                let in_open = self
                    .nodes
                    .get(&neighbor_hash)
                    .map_or(false, |n| n.in_open_set);
                let better_path = !in_open
                    || tentative_g
                        < self
                            .nodes
                            .get(&neighbor_hash)
                            .map_or(f32::INFINITY, |n| n.g);

                if better_path {
                    let neighbor_hash = Self::hash_coords(neighbor.x, neighbor.y, neighbor.z);
                    if let Some(node) = self.nodes.get_mut(&neighbor_hash) {
                        node.came_from = Some(Box::new(current.clone()));
                        node.g = tentative_g;
                        node.h = neighbor.distance_to_sqr(&to) as f32;
                        node.f = node.g + node.h;
                    } else {
                        let mut neighbor_node = neighbor.clone();
                        neighbor_node.came_from = Some(Box::new(current.clone()));
                        neighbor_node.g = tentative_g;
                        neighbor_node.h = neighbor.distance_to_sqr(&to) as f32;
                        neighbor_node.f = neighbor_node.g + neighbor_node.h;
                        self.nodes.insert(
                            Self::hash_coords(neighbor.x, neighbor.y, neighbor.z),
                            neighbor_node.clone(),
                        );
                    }

                    let neighbor_hash = Self::hash_coords(neighbor.x, neighbor.y, neighbor.z);
                    if let Some(node) = self.nodes.get(&neighbor_hash) {
                        if node.in_open_set {
                            self.open_set.change_cost(
                                self.open_set
                                    .find_index(neighbor.x, neighbor.y, neighbor.z)
                                    .unwrap(),
                                node.f,
                            );
                        } else {
                            self.open_set.push(node.clone());
                            if let Some(node) = self.nodes.get_mut(&neighbor_hash) {
                                node.in_open_set = true;
                            }
                        }
                    }
                }
            }
        }

        // No path found, return closest
        if closest.x == from.x && closest.y == from.y && closest.z == from.z {
            None
        } else {
            Some(self.reconstruct_path(&closest))
        }
    }

    fn get_neighbors(
        &mut self,
        level: &SimLevel,
        pos: &PathNode,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        target: &PathNode,
    ) -> usize {
        self.neighbors.clear();

        let jump_size = if self.is_free(level, pos.x, pos.y + 1, pos.z, size_x, size_y, size_z)
            == PathNodeType::Open
        {
            1
        } else {
            0
        };

        let neighbors_coords = [
            (pos.x, pos.y, pos.z + 1), // North
            (pos.x - 1, pos.y, pos.z), // West
            (pos.x + 1, pos.y, pos.z), // East
            (pos.x, pos.y, pos.z - 1), // South
        ];

        let mut count = 0;
        for (x, y, z) in neighbors_coords {
            if let Some(node) = self.get_node(level, x, y, z, size_x, size_y, size_z, jump_size) {
                if !self
                    .nodes
                    .get(&Self::hash_coords(x, y, z))
                    .map_or(false, |n| n.closed)
                    && node.distance_to_sqr(target) < 10000.0
                {
                    // max_dist^2 approximation
                    self.neighbors.push(node);
                    count += 1;
                }
            }
        }
        count
    }

    fn get_node(
        &mut self,
        level: &SimLevel,
        x: i32,
        y: i32,
        z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        jump_size: i32,
    ) -> Option<PathNode> {
        let path_type = self.is_free(level, x, y, z, size_x, size_y, size_z);

        if path_type == PathNodeType::Walkable {
            return Some(self.get_or_create_node(x, y, z));
        }

        let mut best = None;
        if path_type == PathNodeType::Open {
            best = Some(self.get_or_create_node(x, y, z));
        }

        if best.is_none()
            && jump_size > 0
            && path_type != PathNodeType::Fence
            && path_type != PathNodeType::Trap
        {
            if self.is_free(level, x, y + jump_size, z, size_x, size_y, size_z)
                == PathNodeType::Open
            {
                best = Some(self.get_or_create_node(x, y + jump_size, z));
            }
        }

        if let Some(mut node) = best {
            let mut drop = 0;
            let mut current_y = y;
            loop {
                if current_y <= 0 {
                    return None;
                }
                let cost = self.is_free(level, x, current_y - 1, z, size_x, size_y, size_z);
                if self.avoid_water && cost == PathNodeType::Water {
                    return None;
                }
                if cost != PathNodeType::Open {
                    break;
                }
                if drop >= 4 {
                    return None;
                }
                current_y -= 1;
                drop += 1;
                if current_y > 0 {
                    node = self.get_or_create_node(x, current_y, z);
                }
                if cost == PathNodeType::Lava {
                    return None;
                }
            }
            return Some(node);
        }
        None
    }

    fn is_free(
        &self,
        level: &SimLevel,
        x: i32,
        y: i32,
        z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
    ) -> PathNodeType {
        // Check if the entity-sized box at (x,y,z) is free
        for dx in 0..size_x {
            for dy in 0..size_y {
                for dz in 0..size_z {
                    let bx = x + dx;
                    let by = y + dy;
                    let bz = z + dz;

                    if let Some(block) = level.get_block(mclre_world::BlockPos::new(bx, by, bz)) {
                        if block == mclre_core::BlockStateId(0) {
                            continue; // Air
                        }
                        if block == mclre_core::BlockStateId(8)
                            || block == mclre_core::BlockStateId(9)
                        {
                            return PathNodeType::Water;
                        }
                        if block == mclre_core::BlockStateId(10)
                            || block == mclre_core::BlockStateId(11)
                        {
                            return PathNodeType::Lava;
                        }
                        // Check for fences, trapdoors, etc.
                        return PathNodeType::Closed;
                    }
                }
            }
        }
        PathNodeType::Walkable
    }

    fn reconstruct_path(&self, end: &PathNode) -> Path {
        let mut path = Path::new();
        let mut current = Some(end);
        while let Some(node) = current {
            path.nodes.push(node.clone());
            current = node.came_from.as_ref().map(|b| b.as_ref());
        }
        path.nodes.reverse();
        path
    }
}

/// Path navigation for mobs (matching LCE PathNavigation)
#[derive(Debug)]
pub struct PathNavigation {
    pub path: Option<Path>,
    pub speed: f32,
    pub max_dist: f32,
    pub avoid_water: bool,
    pub can_open_doors: bool,
    pub can_pass_doors: bool,
    pub can_float: bool,
    pub avoid_sun: bool,
    path_finder: PathFinder,
    last_path_update: i32,
    last_stuck_check: i32,
    last_stuck_pos: Vec3<f64>,
}

impl PathNavigation {
    pub fn new(max_dist: f32) -> Self {
        Self {
            path: None,
            speed: 0.0,
            max_dist,
            avoid_water: false,
            can_open_doors: false,
            can_pass_doors: false,
            can_float: false,
            avoid_sun: false,
            path_finder: PathFinder::new(false, false, false, false),
            last_path_update: 0,
            last_stuck_check: 0,
            last_stuck_pos: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn set_avoid_water(&mut self, avoid: bool) {
        self.avoid_water = avoid;
        self.path_finder.avoid_water = avoid;
    }

    pub fn set_can_open_doors(&mut self, can: bool) {
        self.can_open_doors = can;
        self.path_finder.can_open_doors = can;
    }

    pub fn set_can_pass_doors(&mut self, can: bool) {
        self.can_pass_doors = can;
        self.path_finder.can_pass_doors = can;
    }

    pub fn set_can_float(&mut self, can: bool) {
        self.can_float = can;
        self.path_finder.can_float = can;
    }

    pub fn move_to_pos(
        &mut self,
        level: &mut SimLevel,
        entity: &mut MobEntity,
        x: f64,
        y: f64,
        z: f64,
    ) -> bool {
        self.path = self
            .path_finder
            .find_path_to_pos(level, entity, x, y, z, self.max_dist);
        self.path.is_some()
    }

    pub fn move_to_entity(
        &mut self,
        level: &mut SimLevel,
        entity: &mut MobEntity,
        target: &MobEntity,
    ) -> bool {
        self.move_to_pos(
            level,
            entity,
            target.entity.position.x,
            target.entity.position.y,
            target.entity.position.z,
        )
    }

    pub fn tick(&mut self, level: &mut SimLevel, entity: &mut MobEntity) {
        if let Some(path) = &mut self.path {
            if path.is_empty() {
                self.path = None;
                return;
            }

            // Check if we're stuck
            if self.last_stuck_check > 100 {
                let dist = (entity.entity.position - self.last_stuck_pos).length();
                if dist < 0.1 {
                    self.path = None;
                    return;
                }
                self.last_stuck_check = 0;
                self.last_stuck_pos = entity.entity.position;
            }
            self.last_stuck_check += 1;

            // Move to next waypoint
            if let Some(next_node) = path.get_next_node() {
                let target_x = next_node.x as f64 + 0.5;
                let target_y = next_node.y as f64 + 0.5;
                let target_z = next_node.z as f64 + 0.5;

                let dx = target_x - entity.entity.position.x;
                let dy = target_y - entity.entity.position.y;
                let dz = target_z - entity.entity.position.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();

                if dist < 0.5 {
                    // Reached waypoint, continue to next
                    return;
                }

                // Move towards waypoint
                let move_speed = self.speed as f64 / dist.max(0.1f64);
                entity.entity.velocity.x += dx * move_speed as f64 * 0.1;
                entity.entity.velocity.y += dy * move_speed as f64 * 0.1;
                entity.entity.velocity.z += dz * move_speed as f64 * 0.1;

                // Update yaw to face target
                let yaw = (dx.atan2(dz)).to_degrees() as f32 - 90.0;
                entity.entity.yaw = yaw;
            }
        } else {
            self.path = None;
        }
    }

    pub fn is_done(&self) -> bool {
        self.path.as_ref().map_or(true, |p| p.is_empty())
    }

    pub fn stop(&mut self) {
        self.path = None;
    }
}

// ============================================================================
// Sensing - for mob awareness
// ============================================================================

#[derive(Debug, Default)]
pub struct Sensing {
    seen_entities: Vec<i32>,
    unseen_entities: Vec<i32>,
}

impl Sensing {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, entity: &MobEntity, level: &SimLevel) {
        // Simplified: just clear and re-scan
        self.seen_entities.clear();
        self.unseen_entities.clear();

        // In a real implementation, this would scan nearby entities
        // and determine which ones are visible
    }
}

// ============================================================================
// MobEntity - base class for all mobs
// ============================================================================

#[derive(Debug)]
pub struct MobEntity {
    pub entity: Entity,
    pub max_health: f32,
    pub health: f32,
    pub move_control: MoveControl,
    pub jump_control: JumpControl,
    pub goal_selector: GoalSelector,
    pub target_selector: GoalSelector,
    pub sensing: Sensing,
    pub attack_target: Option<i32>,
    pub attack_time: i16,
    pub swing_time: i16,
    pub last_hurt_time: i16,
    pub last_damage_source: Option<DamageSource>,
    pub speed: f32,
    pub jump_power: f32,
    pub can_breathe_underwater: bool,
    pub can_fly: bool,
    pub is_baby: bool,
    pub age: i32,
    pub experience_reward: i32,
}

impl MobEntity {
    pub fn new(entity_type: EntityType, id: i32, position: Vec3<f64>) -> Self {
        let entity = Entity::new(entity_type, id, position);
        let move_control = MoveControl::new(id);
        let jump_control = JumpControl::new(id);

        let max_health = match entity_type {
            EntityType::Chicken => 4.0,
            EntityType::Cow => 10.0,
            EntityType::Pig => 10.0,
            EntityType::Sheep => 8.0,
            EntityType::Wolf => 8.0,
            EntityType::Villager => 20.0,
            EntityType::Zombie => 20.0,
            EntityType::Creeper => 20.0,
            EntityType::Skeleton => 20.0,
            EntityType::Spider => 16.0,
            EntityType::Enderman => 40.0,
            EntityType::Ghast => 10.0,
            EntityType::Slime => 16.0,
            EntityType::MagmaCube => 16.0,
            EntityType::Blaze => 20.0,
            EntityType::WitherSkeleton => 20.0,
            EntityType::Witch => 26.0,
            EntityType::EnderDragon => 200.0,
            EntityType::Wither => 300.0,
            _ => 10.0,
        };

        Self {
            entity,
            max_health,
            health: max_health,
            move_control,
            jump_control,
            goal_selector: GoalSelector::new(),
            target_selector: GoalSelector::new(),
            sensing: Sensing::new(),
            attack_target: None,
            attack_time: 0,
            swing_time: 0,
            last_hurt_time: 0,
            last_damage_source: None,
            speed: 0.23,
            jump_power: 0.42,
            can_breathe_underwater: false,
            can_fly: false,
            is_baby: false,
            age: 0,
            experience_reward: match entity_type {
                EntityType::Chicken => 1,
                EntityType::Cow => 1,
                EntityType::Pig => 1,
                EntityType::Sheep => 1,
                EntityType::Wolf => 1,
                EntityType::Villager => 3,
                EntityType::Zombie => 5,
                EntityType::Creeper => 5,
                EntityType::Skeleton => 5,
                EntityType::Spider => 5,
                EntityType::Enderman => 5,
                EntityType::Ghast => 5,
                EntityType::Slime => 1,
                EntityType::MagmaCube => 1,
                EntityType::Blaze => 10,
                EntityType::WitherSkeleton => 5,
                EntityType::Witch => 5,
                EntityType::EnderDragon => 12000,
                EntityType::Wither => 50,
                _ => 1,
            },
        }
    }

    pub fn get_entity(&self) -> &Entity {
        &self.entity
    }

    pub fn get_entity_mut(&mut self) -> &mut Entity {
        &mut self.entity
    }

    pub fn get_id(&self) -> i32 {
        self.entity.id
    }

    pub fn get_type(&self) -> EntityType {
        self.entity.entity_type
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0.0 && !self.entity.removed
    }

    pub fn hurt(&mut self, source: DamageSource, amount: f32) -> bool {
        if !self.is_alive() || self.entity.invulnerable_time > 0 {
            return false;
        }

        let actual_damage = amount.min(self.health);
        self.health -= actual_damage;
        self.entity.invulnerable_time = 20; // 1 second
        self.last_hurt_time = 20;
        self.last_damage_source = Some(source);

        if self.health <= 0.0 {
            self.die(source);
        }

        true
    }

    pub fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).min(self.max_health);
    }

    pub fn die(&mut self, source: DamageSource) {
        self.health = 0.0;
        self.entity.removed = true;
        // Drop loot would happen here
    }

    pub fn tick(&mut self, level: &mut SimLevel) {
        self.entity.tick();

        if self.attack_time > 0 {
            self.attack_time -= 1;
        }
        if self.swing_time > 0 {
            self.swing_time -= 1;
        }
        if self.last_hurt_time > 0 {
            self.last_hurt_time -= 1;
        }

        self.move_control.tick(&mut self.entity, level);
        self.jump_control.tick(&mut self.entity);

        // Tick selectors and sensing - use separate scopes with mem::replace to avoid borrow conflicts
        {
            let mut goal_selector = std::mem::replace(&mut self.goal_selector, GoalSelector::new());
            goal_selector.tick(self, level);
            self.goal_selector = goal_selector;
        }
        {
            let mut target_selector =
                std::mem::replace(&mut self.target_selector, GoalSelector::new());
            target_selector.tick(self, level);
            self.target_selector = target_selector;
        }
        {
            let mut sensing = std::mem::replace(&mut self.sensing, Sensing::new());
            sensing.tick(self, level);
            self.sensing = sensing;
        }

        // Apply gravity
        if self.entity.gravity {
            self.entity.velocity.y -= crate::physics::GRAVITY as f64;
            if self.entity.velocity.y < -crate::physics::TERMINAL_VELOCITY as f64 {
                self.entity.velocity.y = -crate::physics::TERMINAL_VELOCITY as f64;
            }
        }

        // Move entity
        self.entity.position.x += self.entity.velocity.x;
        self.entity.position.y += self.entity.velocity.y;
        self.entity.position.z += self.entity.velocity.z;

        // Update bounding box
        let w = self.entity.width as f64 / 2.0;
        let min = Vec3::new(
            self.entity.position.x - w,
            self.entity.position.y,
            self.entity.position.z - w,
        );
        let max = Vec3::new(
            self.entity.position.x + w,
            self.entity.position.y + self.entity.height as f64,
            self.entity.position.z + w,
        );
        self.entity.bounding_box = AABB::new(min, max);

        // Apply drag
        if self.entity.on_ground {
            self.entity.velocity.x *= 0.6;
            self.entity.velocity.z *= 0.6;
        } else {
            self.entity.velocity.x *= 0.91;
            self.entity.velocity.z *= 0.91;
        }
        self.entity.velocity.x *= crate::physics::DRAG_AIR as f64;
        self.entity.velocity.z *= crate::physics::DRAG_AIR as f64;

        // Age
        self.age += 1;
        if self.is_baby && self.age > 24000 {
            // 20 minutes
            self.is_baby = false;
        }
    }

    pub fn look_at(&mut self, target_x: f64, target_y: f64, target_z: f64) {
        let dx = target_x - self.entity.position.x;
        let dy = target_y - (self.entity.position.y + self.entity.eye_height as f64);
        let dz = target_z - self.entity.position.z;

        let yaw = (dx.atan2(dz)).to_degrees() as f32 - 90.0;
        let pitch = (-(dy / (dx * dx + dz * dz).sqrt())).to_degrees() as f32;

        self.entity.yaw = yaw.clamp(-180.0, 180.0);
        self.entity.pitch = pitch.clamp(-90.0, 90.0);
    }

    pub fn get_look_vector(&self) -> Vec3<f64> {
        let yaw_rad = (self.entity.yaw as f64).to_radians();
        let pitch_rad = (self.entity.pitch as f64).to_radians();
        Vec3::new(
            -yaw_rad.sin() * pitch_rad.cos(),
            -pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        )
    }
}

// ============================================================================
// Specific Mob Implementations
// ============================================================================

/// Passive animal base
pub struct AnimalEntity {
    pub mob: MobEntity,
    pub breed_cooldown: i32,
    pub in_love: bool,
    pub love_time: i32,
}

impl AnimalEntity {
    pub fn new(entity_type: EntityType, id: i32, position: Vec3<f64>) -> Self {
        let mut mob = MobEntity::new(entity_type, id, position);
        // Add animal-specific goals
        // mob.goal_selector.add_goal(0, Box::new(PanicGoal::new()));
        // mob.goal_selector.add_goal(1, Box::new(BreedGoal::new()));
        // mob.goal_selector.add_goal(2, Box::new(FollowParentGoal::new()));
        // mob.goal_selector.add_goal(3, Box::new(WaterAvoidingRandomStrollGoal::new()));
        // mob.goal_selector.add_goal(4, Box::new(LookAtPlayerGoal::new()));
        // mob.goal_selector.add_goal(5, Box::new(RandomLookAroundGoal::new()));

        Self {
            mob,
            breed_cooldown: 0,
            in_love: false,
            love_time: 0,
        }
    }
}

/// Monster base
pub struct MonsterEntity {
    pub mob: MobEntity,
    pub hostile: bool,
}

impl MonsterEntity {
    pub fn new(entity_type: EntityType, id: i32, position: Vec3<f64>) -> Self {
        let mut mob = MobEntity::new(entity_type, id, position);
        // Add monster-specific goals
        // mob.goal_selector.add_goal(0, Box::new(MeleeAttackGoal::new()));
        // mob.goal_selector.add_goal(1, Box::new(FloatGoal::new()));
        // mob.target_selector.add_goal(0, Box::new(HurtByTargetGoal::new()));
        // mob.target_selector.add_goal(1, Box::new(NearestAttackableTargetGoal::new()));

        Self { mob, hostile: true }
    }
}

/// Zombie-specific
pub struct ZombieEntity {
    pub monster: MonsterEntity,
    pub can_break_doors: bool,
    pub conversion_time: i32,
}

impl ZombieEntity {
    pub fn new(id: i32, position: Vec3<f64>) -> Self {
        let monster = MonsterEntity::new(EntityType::Zombie, id, position);
        Self {
            monster,
            can_break_doors: false,
            conversion_time: -1,
        }
    }
}

/// Skeleton-specific
pub struct SkeletonEntity {
    pub monster: MonsterEntity,
    pub ranged_attack: bool,
}

impl SkeletonEntity {
    pub fn new(id: i32, position: Vec3<f64>) -> Self {
        let monster = MonsterEntity::new(EntityType::Skeleton, id, position);
        Self {
            monster,
            ranged_attack: true,
        }
    }
}

/// Creeper-specific
pub struct CreeperEntity {
    pub monster: MonsterEntity,
    pub explosion_radius: f32,
    pub fuse_time: i16,
    pub max_fuse_time: i16,
    pub powered: bool,
}

impl CreeperEntity {
    pub fn new(id: i32, position: Vec3<f64>) -> Self {
        let monster = MonsterEntity::new(EntityType::Creeper, id, position);
        Self {
            monster,
            explosion_radius: 3.0,
            fuse_time: 30,
            max_fuse_time: 30,
            powered: false,
        }
    }
}

/// Spider-specific
pub struct SpiderEntity {
    pub monster: MonsterEntity,
    pub climbing: bool,
}

impl SpiderEntity {
    pub fn new(id: i32, position: Vec3<f64>) -> Self {
        let monster = MonsterEntity::new(EntityType::Spider, id, position);
        Self {
            monster,
            climbing: false,
        }
    }
}

/// Enderman-specific
pub struct EndermanEntity {
    pub monster: MonsterEntity,
    pub carried_block: Option<BlockStateId>,
}

impl EndermanEntity {
    pub fn new(id: i32, position: Vec3<f64>) -> Self {
        let monster = MonsterEntity::new(EntityType::Enderman, id, position);
        Self {
            monster,
            carried_block: None,
        }
    }
}

/// Villager-specific
pub struct VillagerEntity {
    pub mob: MobEntity,
    pub profession: VillagerProfession,
    pub level: i32,
    pub trades: Vec<VillagerTrade>,
    pub restock_timer: i32,
    pub gossip: Vec<VillagerGossip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillagerProfession {
    None = 0,
    Farmer = 1,
    Librarian = 2,
    Priest = 3,
    Blacksmith = 4,
    Butcher = 5,
    Nitwit = 6,
}

#[derive(Debug, Clone)]
pub struct VillagerTrade {
    pub buy_item: BlockStateId,
    pub buy_count: i32,
    pub sell_item: BlockStateId,
    pub sell_count: i32,
    pub max_uses: i32,
    pub uses: i32,
    pub price_multiplier: f32,
    pub xp: i32,
}

#[derive(Debug, Clone)]
pub struct VillagerGossip {
    pub target: i32, // player UUID or entity ID
    pub gossip_type: GossipType,
    pub value: i32,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipType {
    MajorPositive,
    MinorPositive,
    MajorNegative,
    MinorNegative,
}

impl VillagerEntity {
    pub fn new(id: i32, position: Vec3<f64>) -> Self {
        let mut mob = MobEntity::new(EntityType::Villager, id, position);
        Self {
            mob,
            profession: VillagerProfession::None,
            level: 1,
            trades: Vec::new(),
            restock_timer: 0,
            gossip: Vec::new(),
        }
    }
}

// ============================================================================
// Entity Manager / Spawning
// ============================================================================

#[derive(Debug, Clone)]
pub struct MobSpawnInfo {
    pub entity_type: EntityType,
    pub weight: i32,
    pub min_count: i32,
    pub max_count: i32,
    pub biome_categories: Vec<MobCategory>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MobCategory {
    Monster,
    Creature,
    Ambient,
    WaterCreature,
    WaterAmbient,
    Misc,
}

pub struct MobSpawner {
    spawn_infos: Vec<MobSpawnInfo>,
    random: GameRandom,
}

impl MobSpawner {
    pub fn new(seed: i64) -> Self {
        let mut spawner = Self {
            spawn_infos: Vec::new(),
            random: GameRandom::with_seed(seed),
        };
        spawner.register_default_spawns();
        spawner
    }

    fn register_default_spawns(&mut self) {
        // Monster spawns (hostile)
        self.add_spawn(EntityType::Zombie, 100, 4, 4, vec![MobCategory::Monster]);
        self.add_spawn(EntityType::Creeper, 100, 1, 4, vec![MobCategory::Monster]);
        self.add_spawn(EntityType::Skeleton, 100, 1, 4, vec![MobCategory::Monster]);
        self.add_spawn(EntityType::Spider, 100, 1, 4, vec![MobCategory::Monster]);
        self.add_spawn(EntityType::Enderman, 10, 1, 4, vec![MobCategory::Monster]);
        self.add_spawn(EntityType::Witch, 5, 1, 1, vec![MobCategory::Monster]);
        self.add_spawn(EntityType::Slime, 50, 1, 4, vec![MobCategory::Monster]);

        // Animal spawns
        self.add_spawn(EntityType::Cow, 100, 4, 4, vec![MobCategory::Creature]);
        self.add_spawn(EntityType::Pig, 100, 4, 4, vec![MobCategory::Creature]);
        self.add_spawn(EntityType::Sheep, 100, 4, 4, vec![MobCategory::Creature]);
        self.add_spawn(EntityType::Chicken, 100, 4, 4, vec![MobCategory::Creature]);

        // Water creatures
        self.add_spawn(
            EntityType::Squid,
            10,
            1,
            4,
            vec![MobCategory::WaterCreature],
        );
    }

    fn add_spawn(
        &mut self,
        entity_type: EntityType,
        weight: i32,
        min_count: i32,
        max_count: i32,
        categories: Vec<MobCategory>,
    ) {
        self.spawn_infos.push(MobSpawnInfo {
            entity_type,
            weight,
            min_count,
            max_count,
            biome_categories: categories,
        });
    }

    pub fn can_spawn(
        &self,
        entity_type: EntityType,
        category: MobCategory,
        light_level: u8,
        block_below: BlockStateId,
    ) -> bool {
        // Simplified spawn checks
        let info = self
            .spawn_infos
            .iter()
            .find(|i| i.entity_type == entity_type);
        if let Some(info) = info {
            if !info.biome_categories.contains(&category) {
                return false;
            }
            // Hostile mobs need low light
            if category == MobCategory::Monster && light_level > 7 {
                return false;
            }
            // Passive mobs need grass/light
            if category == MobCategory::Creature && light_level < 9 {
                return false;
            }
            true
        } else {
            false
        }
    }

    pub fn try_spawn(
        &mut self,
        category: MobCategory,
        position: Vec3<f64>,
        level: &SimLevel,
    ) -> Option<MobEntity> {
        // Filter eligible spawns
        let eligible: Vec<_> = self
            .spawn_infos
            .iter()
            .filter(|i| i.biome_categories.contains(&category))
            .collect();

        if eligible.is_empty() {
            return None;
        }

        // Weighted random selection
        let total_weight: i32 = eligible.iter().map(|i| i.weight).sum();
        let mut roll = self.random.next_int_bound(total_weight) + 1;

        for info in eligible {
            roll -= info.weight;
            if roll <= 0 {
                let count = self
                    .random
                    .next_int_bound(info.max_count - info.min_count + 1)
                    + info.min_count;
                // For now, just spawn one
                return Some(MobEntity::new(info.entity_type, 0, position));
            }
        }
        None
    }
}

// ============================================================================
// SimLevel extension for entity support
// ============================================================================

// This would be in sim.rs or a separate file
pub struct SimLevel {
    // ... existing fields ...
    pub entities: Vec<MobEntity>,
    pub entity_counter: i32,
    pub mob_spawner: MobSpawner,
    pub rng: mclre_core::GameRandom,
}

impl SimLevel {
    pub fn new(seed: i64) -> Self {
        Self {
            entities: Vec::new(),
            entity_counter: 0,
            mob_spawner: MobSpawner::new(seed),
            rng: mclre_core::GameRandom::with_seed(seed),
        }
    }

    pub fn rng(&mut self) -> &mut mclre_core::GameRandom {
        &mut self.rng
    }

    pub fn add_entity(&mut self, mut entity: MobEntity) -> i32 {
        entity.entity.id = self.entity_counter;
        self.entity_counter += 1;
        let id = entity.entity.id;
        self.entities.push(entity);
        id
    }

    pub fn remove_entity(&mut self, id: i32) {
        self.entities.retain(|e| e.entity.id != id);
    }

    pub fn get_entity(&self, id: i32) -> Option<&MobEntity> {
        self.entities.iter().find(|e| e.entity.id == id)
    }

    pub fn get_entity_mut(&mut self, id: i32) -> Option<&mut MobEntity> {
        self.entities.iter_mut().find(|e| e.entity.id == id)
    }

    pub fn get_block(&self, pos: mclre_world::BlockPos) -> Option<mclre_core::BlockStateId> {
        // Simplified: return air for now
        // In a real implementation, this would query the chunk storage
        Some(mclre_core::BlockStateId(0))
    }

    pub fn tick_entities(&mut self) {
        // Update all entities - extract, tick, and reinsert to avoid borrow conflicts
        let mut entities = std::mem::take(&mut self.entities);
        for entity in &mut entities {
            entity.tick(self);
        }
        self.entities = entities;

        // Remove dead entities
        self.entities.retain(|e| e.is_alive());

        // Spawn mobs
        self.tick_spawning();
    }

    fn tick_spawning(&mut self) {
        // Simplified: try to spawn monsters around players
        // In a real implementation, this would check spawn conditions per chunk
    }
}
