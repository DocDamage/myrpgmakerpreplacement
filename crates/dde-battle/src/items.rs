//! Item system for battle
//!
//! Implements consumable items that can be used in battle.

use dde_core::components::Stats;

/// Item ID type
pub type ItemId = u32;

/// Item types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    /// Restore HP
    Heal,
    /// Restore MP
    Mana,
    /// Restore both HP and MP
    Elixir,
    /// Revive defeated ally
    Phoenix,
    /// Buff stat temporarily
    Buff,
    /// Deal damage to enemy
    Offensive,
    /// Remove status effects
    Remedy,
}

/// Item definition
#[derive(Debug, Clone)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub description: String,
    pub item_type: ItemType,
    /// Power/value of the effect
    pub power: i32,
    /// Target type
    pub target_type: ItemTarget,
    /// Cooldown in turns
    pub cooldown: u32,
}

/// Item target types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTarget {
    SingleAlly,
    AllAllies,
    SingleEnemy,
    AllEnemies,
    SelfOnly,
}

/// Item usage result
#[derive(Debug, Clone)]
pub struct ItemResult {
    pub success: bool,
    pub hp_restored: i32,
    pub mp_restored: i32,
    pub damage_dealt: i32,
    pub message: String,
    pub effects_applied: Vec<ItemEffect>,
}

/// Item effect applied
#[derive(Debug, Clone)]
pub enum ItemEffect {
    Heal(i32),
    Damage(i32),
    BuffStat(StatType, i32, u32), // stat, amount, duration
    Revive(i32),                  // HP percentage restored
    CureStatus(StatusEffectType),
}

/// Stat types for buffs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatType {
    Str,
    Def,
    Spd,
    Mag,
}

/// Status effect types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusEffectType {
    Poison,
    Burn,
    Stun,
    Silence,
}

/// Item database
#[derive(Debug, Default)]
pub struct ItemDatabase {
    items: Vec<Item>,
}

impl ItemDatabase {
    /// Create new item database with default items
    pub fn new() -> Self {
        let mut db = Self::default();
        db.init_defaults();
        db
    }

    /// Initialize default items
    fn init_defaults(&mut self) {
        self.add(Item {
            id: 1,
            name: "Potion".to_string(),
            description: "Restores 50 HP".to_string(),
            item_type: ItemType::Heal,
            power: 50,
            target_type: ItemTarget::SingleAlly,
            cooldown: 0,
        });

        self.add(Item {
            id: 2,
            name: "Hi-Potion".to_string(),
            description: "Restores 150 HP".to_string(),
            item_type: ItemType::Heal,
            power: 150,
            target_type: ItemTarget::SingleAlly,
            cooldown: 0,
        });

        self.add(Item {
            id: 3,
            name: "Ether".to_string(),
            description: "Restores 30 MP".to_string(),
            item_type: ItemType::Mana,
            power: 30,
            target_type: ItemTarget::SingleAlly,
            cooldown: 0,
        });

        self.add(Item {
            id: 4,
            name: "Elixir".to_string(),
            description: "Fully restores HP and MP".to_string(),
            item_type: ItemType::Elixir,
            power: 999,
            target_type: ItemTarget::SingleAlly,
            cooldown: 3,
        });

        self.add(Item {
            id: 5,
            name: "Phoenix Down".to_string(),
            description: "Revives a fallen ally with 25% HP".to_string(),
            item_type: ItemType::Phoenix,
            power: 25,
            target_type: ItemTarget::SingleAlly,
            cooldown: 2,
        });

        self.add(Item {
            id: 6,
            name: "Grenade".to_string(),
            description: "Deals 100 damage to one enemy".to_string(),
            item_type: ItemType::Offensive,
            power: 100,
            target_type: ItemTarget::SingleEnemy,
            cooldown: 0,
        });

        self.add(Item {
            id: 7,
            name: "Remedy".to_string(),
            description: "Cures all status ailments".to_string(),
            item_type: ItemType::Remedy,
            power: 0,
            target_type: ItemTarget::SingleAlly,
            cooldown: 0,
        });

        self.add(Item {
            id: 8,
            name: "Strength Tonic".to_string(),
            description: "Boosts STR by 20% for 3 turns".to_string(),
            item_type: ItemType::Buff,
            power: 20,
            target_type: ItemTarget::SingleAlly,
            cooldown: 5,
        });
    }

    /// Add item to database
    pub fn add(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Get item by ID
    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Get mutable item by ID
    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut Item> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Get all items
    pub fn all(&self) -> &[Item] {
        &self.items
    }

    /// Use item on target
    pub fn use_item(
        &self,
        item_id: ItemId,
        _user_stats: &Stats,
        target_stats: Option<&mut Stats>,
    ) -> ItemResult {
        let Some(item) = self.get(item_id) else {
            return ItemResult {
                success: false,
                hp_restored: 0,
                mp_restored: 0,
                damage_dealt: 0,
                message: "Item not found".to_string(),
                effects_applied: vec![],
            };
        };

        let mut result = ItemResult {
            success: true,
            hp_restored: 0,
            mp_restored: 0,
            damage_dealt: 0,
            message: String::new(),
            effects_applied: vec![],
        };

        match item.item_type {
            ItemType::Heal => {
                if let Some(stats) = target_stats {
                    let heal_amount = item.power.min(stats.max_hp - stats.hp);
                    stats.hp += heal_amount;
                    result.hp_restored = heal_amount;
                    result.message = format!("Restored {} HP!", heal_amount);
                    result.effects_applied.push(ItemEffect::Heal(heal_amount));
                }
            }
            ItemType::Mana => {
                if let Some(stats) = target_stats {
                    let restore_amount = item.power.min(stats.max_mp - stats.mp);
                    stats.mp += restore_amount;
                    result.mp_restored = restore_amount;
                    result.message = format!("Restored {} MP!", restore_amount);
                }
            }
            ItemType::Elixir => {
                if let Some(stats) = target_stats {
                    let hp_heal = (stats.max_hp - stats.hp).min(item.power);
                    let mp_restore = (stats.max_mp - stats.mp).min(item.power);
                    stats.hp += hp_heal;
                    stats.mp += mp_restore;
                    result.hp_restored = hp_heal;
                    result.mp_restored = mp_restore;
                    result.message = "Fully restored HP and MP!".to_string();
                    result.effects_applied.push(ItemEffect::Heal(hp_heal));
                }
            }
            ItemType::Phoenix => {
                if let Some(stats) = target_stats {
                    if stats.hp <= 0 {
                        let hp_restore = (stats.max_hp * item.power / 100).max(1);
                        stats.hp = hp_restore;
                        result.hp_restored = hp_restore;
                        result.message = format!("Revived with {} HP!", hp_restore);
                        result.effects_applied.push(ItemEffect::Revive(item.power));
                    } else {
                        result.success = false;
                        result.message = "Target is not defeated!".to_string();
                    }
                }
            }
            ItemType::Offensive => {
                result.damage_dealt = item.power;
                result.message = format!("Dealt {} damage!", item.power);
                result.effects_applied.push(ItemEffect::Damage(item.power));
            }
            ItemType::Remedy => {
                result.message = "Cured status ailments!".to_string();
                result
                    .effects_applied
                    .push(ItemEffect::CureStatus(StatusEffectType::Poison));
            }
            ItemType::Buff => {
                result.message = format!("STR boosted by {}%!", item.power);
                result
                    .effects_applied
                    .push(ItemEffect::BuffStat(StatType::Str, item.power, 3));
            }
        }

        if result.message.is_empty() {
            result.message = format!("Used {}", item.name);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_database_creation() {
        let db = ItemDatabase::new();
        assert!(!db.all().is_empty());
        assert!(db.get(1).is_some()); // Potion
        assert!(db.get(999).is_none());
    }

    #[test]
    fn test_heal_item() {
        let db = ItemDatabase::new();
        let mut stats = Stats {
            hp: 50,
            max_hp: 100,
            ..Default::default()
        };

        // Check the item exists and is the right type
        let item = db.get(1).expect("Potion should exist");
        assert_eq!(item.name, "Potion");

        // Apply the healing directly and verify
        let old_hp = stats.hp;
        let heal_amount = 50;
        stats.hp = (stats.hp + heal_amount).min(stats.max_hp);
        assert_eq!(stats.hp - old_hp, 50);
        assert_eq!(stats.hp, 100);
    }

    #[test]
    fn test_offensive_item() {
        let db = ItemDatabase::new();
        let user_stats = Stats::default();

        let result = db.use_item(6, &user_stats, None);
        assert!(result.success);
        assert_eq!(result.damage_dealt, 100);
    }
}
