//! Achievements and Statistics tracking for LCE baseline profile.
//!
//! Implements LCE-style achievements (trophies) and statistics tracking
//! with local persistence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Achievement ID matching LCE achievement names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementId {
    // Early game
    GettingWood,
    Benchmarking,
    TimeToMine,
    HotTopic,
    AcquireHardware,
    Upgrade,
    DeliciousFish,
    OnARail,
    TimeToFarm,
    BakeBread,
    TheLie,
    // Nether
    IntoTheNether,
    ReturnToSender,
    IntoFire,
    LocalBrewery,
    WitheringHeights,
    // End
    TheEnd,
    TheEndAgain,
    // Exploration
    AdventuringTime,
    VeryVeryFrightening,
    SniperDuel,
    // Misc
    Diamonds,
    DiamondsToYou,
    Overkill,
    Librarian,
    // LCE-specific
    TutorialComplete,
    SplitScreenPlay,
    HostPrivileges,
    DlcPack,
}

impl AchievementId {
    pub const ALL: &'static [AchievementId] = &[
        AchievementId::GettingWood,
        AchievementId::Benchmarking,
        AchievementId::TimeToMine,
        AchievementId::HotTopic,
        AchievementId::AcquireHardware,
        AchievementId::Upgrade,
        AchievementId::DeliciousFish,
        AchievementId::OnARail,
        AchievementId::TimeToFarm,
        AchievementId::BakeBread,
        AchievementId::TheLie,
        AchievementId::IntoTheNether,
        AchievementId::ReturnToSender,
        AchievementId::IntoFire,
        AchievementId::LocalBrewery,
        AchievementId::WitheringHeights,
        AchievementId::TheEnd,
        AchievementId::TheEndAgain,
        AchievementId::AdventuringTime,
        AchievementId::VeryVeryFrightening,
        AchievementId::SniperDuel,
        AchievementId::Diamonds,
        AchievementId::DiamondsToYou,
        AchievementId::Overkill,
        AchievementId::Librarian,
        AchievementId::TutorialComplete,
        AchievementId::SplitScreenPlay,
        AchievementId::HostPrivileges,
        AchievementId::DlcPack,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            AchievementId::GettingWood => "Getting Wood",
            AchievementId::Benchmarking => "Benchmarking",
            AchievementId::TimeToMine => "Time to Mine!",
            AchievementId::HotTopic => "Hot Topic",
            AchievementId::AcquireHardware => "Acquire Hardware",
            AchievementId::Upgrade => "Upgrade",
            AchievementId::DeliciousFish => "Delicious Fish",
            AchievementId::OnARail => "On A Rail",
            AchievementId::TimeToFarm => "Time to Farm",
            AchievementId::BakeBread => "Bake Bread",
            AchievementId::TheLie => "The Lie",
            AchievementId::IntoTheNether => "Into the Nether",
            AchievementId::ReturnToSender => "Return to Sender",
            AchievementId::IntoFire => "Into Fire",
            AchievementId::LocalBrewery => "Local Brewery",
            AchievementId::WitheringHeights => "Withering Heights",
            AchievementId::TheEnd => "The End?",
            AchievementId::TheEndAgain => "The End... Again...",
            AchievementId::AdventuringTime => "Adventuring Time",
            AchievementId::VeryVeryFrightening => "Very Very Frightening",
            AchievementId::SniperDuel => "Sniper Duel",
            AchievementId::Diamonds => "Diamonds!",
            AchievementId::DiamondsToYou => "Diamonds to You!",
            AchievementId::Overkill => "Overkill",
            AchievementId::Librarian => "Librarian",
            AchievementId::TutorialComplete => "Tutorial Complete",
            AchievementId::SplitScreenPlay => "Split Screen Play",
            AchievementId::HostPrivileges => "Host Privileges",
            AchievementId::DlcPack => "DLC Pack",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AchievementId::GettingWood => "Punch a tree until the block of wood pops out.",
            AchievementId::Benchmarking => "Craft a workbench with four blocks of wooden planks.",
            AchievementId::TimeToMine => "Use planks and sticks to make a pickaxe.",
            AchievementId::HotTopic => "Construct a furnace out of eight cobblestone blocks.",
            AchievementId::AcquireHardware => "Smelt an iron ingot.",
            AchievementId::Upgrade => "Construct a better pickaxe.",
            AchievementId::DeliciousFish => "Catch and cook a fish.",
            AchievementId::OnARail => "Travel by rail 500 meters in a single direction.",
            AchievementId::TimeToFarm => "Hoe and farm a patch of land.",
            AchievementId::BakeBread => "Turn wheat into bread.",
            AchievementId::TheLie => "Bake a cake using wheat, sugar, milk and eggs.",
            AchievementId::IntoTheNether => "Construct a Nether Portal.",
            AchievementId::ReturnToSender => "Destroy a Ghast with a fireball.",
            AchievementId::IntoFire => "Relieve a Blaze of its rod.",
            AchievementId::LocalBrewery => "Brew a potion.",
            AchievementId::WitheringHeights => "Summon the Wither.",
            AchievementId::TheEnd => "Enter the End Portal.",
            AchievementId::TheEndAgain => "Defeat the Ender Dragon.",
            AchievementId::AdventuringTime => "Discover all biomes.",
            AchievementId::VeryVeryFrightening => "Strike a villager with lightning.",
            AchievementId::SniperDuel => "Kill a skeleton with an arrow from more than 50 meters.",
            AchievementId::Diamonds => "Acquire diamonds.",
            AchievementId::DiamondsToYou => "Throw diamonds at another player.",
            AchievementId::Overkill => "Deal nine hearts of damage in a single hit.",
            AchievementId::Librarian => "Buy something from a villager.",
            AchievementId::TutorialComplete => "Complete the tutorial world.",
            AchievementId::SplitScreenPlay => "Play split-screen with a friend.",
            AchievementId::HostPrivileges => "Use host privileges in a game.",
            AchievementId::DlcPack => "Play with a DLC pack.",
        }
    }

    pub fn tier(&self) -> AchievementTier {
        match self {
            AchievementId::GettingWood
            | AchievementId::Benchmarking
            | AchievementId::TimeToMine
            | AchievementId::HotTopic
            | AchievementId::AcquireHardware
            | AchievementId::Upgrade
            | AchievementId::DeliciousFish
            | AchievementId::TimeToFarm
            | AchievementId::BakeBread => AchievementTier::Bronze,
            AchievementId::OnARail
            | AchievementId::TheLie
            | AchievementId::IntoTheNether
            | AchievementId::ReturnToSender
            | AchievementId::IntoFire
            | AchievementId::LocalBrewery
            | AchievementId::Diamonds
            | AchievementId::DiamondsToYou
            | AchievementId::Overkill
            | AchievementId::Librarian => AchievementTier::Silver,
            AchievementId::WitheringHeights
            | AchievementId::TheEnd
            | AchievementId::TheEndAgain
            | AchievementId::AdventuringTime
            | AchievementId::VeryVeryFrightening
            | AchievementId::SniperDuel => AchievementTier::Gold,
            AchievementId::TutorialComplete
            | AchievementId::SplitScreenPlay
            | AchievementId::HostPrivileges
            | AchievementId::DlcPack => AchievementTier::Platinum,
        }
    }

    pub fn gamerscore(&self) -> u32 {
        match self.tier() {
            AchievementTier::Bronze => 15,
            AchievementTier::Silver => 30,
            AchievementTier::Gold => 50,
            AchievementTier::Platinum => 100,
        }
    }
}

/// Achievement tier (Bronze, Silver, Gold, Platinum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AchievementTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

/// Achievement progress for a player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementProgress {
    pub achieved: bool,
    pub timestamp: Option<u64>, // Unix timestamp when achieved
    pub progress: u32,          // For incremental achievements (0-100)
}

/// Statistics categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatCategory {
    General,
    Blocks,
    Items,
    Mobs,
    Custom,
}

/// A single statistic entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistic {
    pub category: StatCategory,
    pub name: String,
    pub value: u64,
}

/// Player statistics tracker
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlayerStatistics {
    pub stats: HashMap<String, u64>,
}

impl PlayerStatistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment(&mut self, name: &str, amount: u64) {
        *self.stats.entry(name.to_string()).or_insert(0) += amount;
    }

    pub fn get(&self, name: &str) -> u64 {
        self.stats.get(name).copied().unwrap_or(0)
    }

    pub fn set(&mut self, name: &str, value: u64) {
        self.stats.insert(name.to_string(), value);
    }
}

/// Achievements manager for a player
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlayerAchievements {
    pub progress: HashMap<AchievementId, AchievementProgress>,
}

impl PlayerAchievements {
    pub fn new() -> Self {
        let mut progress = HashMap::new();
        for achievement in AchievementId::ALL {
            progress.insert(
                *achievement,
                AchievementProgress {
                    achieved: false,
                    timestamp: None,
                    progress: 0,
                },
            );
        }
        Self { progress }
    }

    pub fn is_achieved(&self, id: AchievementId) -> bool {
        self.progress.get(&id).map(|p| p.achieved).unwrap_or(false)
    }

    pub fn unlock(&mut self, id: AchievementId, timestamp: u64) -> bool {
        if let Some(progress) = self.progress.get_mut(&id) {
            if !progress.achieved {
                progress.achieved = true;
                progress.timestamp = Some(timestamp);
                progress.progress = 100;
                return true;
            }
        }
        false
    }

    pub fn set_progress(&mut self, id: AchievementId, progress: u32) {
        if let Some(p) = self.progress.get_mut(&id) {
            if !p.achieved {
                p.progress = progress.min(100);
                if p.progress >= 100 {
                    p.achieved = true;
                    p.timestamp = Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    );
                }
            }
        }
    }

    pub fn get_progress(&self, id: AchievementId) -> u32 {
        self.progress.get(&id).map(|p| p.progress).unwrap_or(0)
    }

    pub fn achieved_count(&self) -> usize {
        self.progress.values().filter(|p| p.achieved).count()
    }

    pub fn total_gamerscore(&self) -> u32 {
        self.progress
            .iter()
            .filter(|(_, p)| p.achieved)
            .map(|(id, _)| id.gamerscore())
            .sum()
    }
}

/// Combined player profile with stats and achievements
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub player_name: String,
    pub uuid: String,
    pub statistics: PlayerStatistics,
    pub achievements: PlayerAchievements,
    pub tutorial_step: Option<u8>, // Current tutorial step (0-20)
    pub last_played: Option<u64>,
    pub play_time_ticks: u64, // Total play time in ticks (20 TPS)
}

impl PlayerProfile {
    pub fn new(player_name: String, uuid: String) -> Self {
        Self {
            player_name,
            uuid,
            statistics: PlayerStatistics::new(),
            achievements: PlayerAchievements::new(),
            tutorial_step: Some(0),
            last_played: None,
            play_time_ticks: 0,
        }
    }

    pub fn increment_stat(&mut self, name: &str, amount: u64) {
        self.statistics.increment(name, amount);
    }

    pub fn unlock_achievement(&mut self, id: AchievementId) -> bool {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.achievements.unlock(id, timestamp)
    }

    pub fn add_play_time(&mut self, ticks: u64) {
        self.play_time_ticks += ticks;
    }

    pub fn play_time_seconds(&self) -> u64 {
        self.play_time_ticks / 20
    }
}

/// Global achievements registry (thread-safe)
pub struct AchievementsRegistry {
    profiles: RwLock<HashMap<String, PlayerProfile>>, // Keyed by UUID
}

impl AchievementsRegistry {
    pub fn new() -> Self {
        Self {
            profiles: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_profile(&self, uuid: &str) -> Option<PlayerProfile> {
        self.profiles.read().unwrap().get(uuid).cloned()
    }

    pub fn create_profile(&self, player_name: String, uuid: String) -> PlayerProfile {
        let profile = PlayerProfile::new(player_name, uuid.clone());
        self.profiles.write().unwrap().insert(uuid, profile.clone());
        profile
    }

    pub fn update_profile(&self, profile: PlayerProfile) {
        self.profiles
            .write()
            .unwrap()
            .insert(profile.uuid.clone(), profile);
    }

    pub fn increment_stat(&self, uuid: &str, name: &str, amount: u64) {
        if let Some(mut profile) = self.get_profile(uuid) {
            profile.increment_stat(name, amount);
            self.update_profile(profile);
        }
    }

    pub fn unlock_achievement(&self, uuid: &str, id: AchievementId) -> bool {
        if let Some(mut profile) = self.get_profile(uuid) {
            let result = profile.unlock_achievement(id);
            self.update_profile(profile);
            result
        } else {
            false
        }
    }
}

impl Default for AchievementsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_unlock() {
        let mut achievements = PlayerAchievements::new();
        assert!(!achievements.is_achieved(AchievementId::GettingWood));
        let timestamp = 1000;
        assert!(achievements.unlock(AchievementId::GettingWood, timestamp));
        assert!(achievements.is_achieved(AchievementId::GettingWood));
        assert!(!achievements.unlock(AchievementId::GettingWood, timestamp)); // Already unlocked
    }

    #[test]
    fn test_gamerscore() {
        let mut achievements = PlayerAchievements::new();
        achievements.unlock(AchievementId::GettingWood, 1000); // 15
        achievements.unlock(AchievementId::Benchmarking, 1000); // 15
        achievements.unlock(AchievementId::TheEnd, 1000); // 50
        assert_eq!(achievements.total_gamerscore(), 80);
    }

    #[test]
    fn test_player_profile() {
        let mut profile = PlayerProfile::new("Steve".to_string(), "test-uuid".to_string());
        profile.increment_stat("walk_one_cm", 100);
        assert_eq!(profile.statistics.get("walk_one_cm"), 100);
        assert!(profile.unlock_achievement(AchievementId::GettingWood));
    }
}
