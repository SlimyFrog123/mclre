//! Core gameplay mechanics for MCLRE baseline profile.

pub mod block_breaking;
pub mod crafting;
pub mod health_hunger;
pub mod inventory;
pub mod fluids;
pub mod fire;
pub mod tnt;
pub mod redstone;
pub mod lighting;
pub mod xp;

/// Tick all gameplay systems in the correct order.
pub fn tick_all(world: &mut crate::SimWorld) -> Result<(), crate::SimError> {
    // Order matters - these must match reference tick order
    crate::gameplay::lighting::tick(world)?;
    crate::gameplay::fluids::tick(world)?;
    crate::gameplay::fire::tick(world)?;
    crate::gameplay::redstone::tick(world)?;
    crate::gameplay::tnt::tick(world)?;
    
    // Block breaking/updates
    crate::gameplay::block_breaking::tick(world)?;
    
    // Health/hunger
    crate::gameplay::health_hunger::tick(world)?;
    
    // XP
    crate::gameplay::xp::tick(world)?;
    
    Ok(())
}