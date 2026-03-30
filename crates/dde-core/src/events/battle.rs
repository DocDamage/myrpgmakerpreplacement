//! Battle Event Types
//!
//! This module defines all events related to the battle system.
//! These events are published through the event bus and can be
//! consumed by quest systems, UI systems, audio systems, etc.
//!
//! # Example
//!
//! ```
//! use dde_core::events::EventBus;
//! use dde_core::events::battle::BattleEvent;
//! use dde_core::events::{Event, EventPriority, EventFilter, EventType, downcast_event};
//!
//! let bus = EventBus::new();
//!
//! // Subscribe to battle events for quest tracking
//! bus.subscribe(
//!     EventFilter::Type(EventType::Battle),
//!     |event| {
//!         if let Some(battle) = downcast_event::<BattleEvent>(event) {
//!             match battle {
//!                 BattleEvent::EnemyDefeated { xp_gained, .. } => {
//!                     println!("Quest progress: defeated enemy, gained {} XP", xp_gained);
//!                 }
//!                 _ => {}
//!             }
//!         }
//!     }
//! );
//!
//! // Publish a battle event
//! bus.publish(
//!     BattleEvent::BattleStarted { encounter_id: 42 },
//!     EventPriority::High
//! );
//! ```

use crate::events::{Event, EventType};
use hecs::Entity;
use std::any::Any;

/// All events related to the battle system
///
/// These events are published during battle encounters and can be
/// used by various systems to react to battle state changes.
#[derive(Debug, Clone)]
pub enum BattleEvent {
    /// Battle has started
    BattleStarted {
        /// Unique identifier for this encounter
        encounter_id: u32,
    },

    /// A new turn has started for an entity
    TurnStarted {
        /// Entity whose turn is starting
        entity: Entity,
    },

    /// A turn has ended for an entity
    TurnEnded {
        /// Entity whose turn has ended
        entity: Entity,
    },

    /// Damage has been dealt to an entity
    DamageDealt {
        /// Entity that dealt the damage
        attacker: Entity,
        /// Entity that received the damage
        defender: Entity,
        /// Amount of damage dealt
        amount: u32,
        /// Whether this was a critical hit
        critical: bool,
    },

    /// Healing has been applied to an entity
    HealingApplied {
        /// Entity that applied the healing (may be same as target)
        source: Entity,
        /// Entity that received healing
        target: Entity,
        /// Amount of healing applied
        amount: u32,
    },

    /// An enemy has been defeated
    EnemyDefeated {
        /// Entity that was defeated
        entity: Entity,
        /// XP gained from defeating this enemy
        xp_gained: u32,
    },

    /// An item has been used
    ItemUsed {
        /// Entity that used the item
        user: Entity,
        /// ID of the item used
        item_id: u32,
        /// Target entity (if any)
        target: Option<Entity>,
    },

    /// A skill has been used
    SkillUsed {
        /// Entity that used the skill
        user: Entity,
        /// ID of the skill used
        skill_id: u32,
        /// Target entity (if any)
        target: Option<Entity>,
    },

    /// Battle has ended
    BattleEnded {
        /// Whether the player won the battle
        victory: bool,
        /// Additional rewards (gold, items, etc.)
        rewards: BattleRewards,
    },

    /// An entity has fled from battle
    FleeAttempt {
        /// Entity attempting to flee
        entity: Entity,
        /// Whether the flee was successful
        success: bool,
    },

    /// A status effect has been applied
    StatusEffectApplied {
        /// Entity receiving the status effect
        target: Entity,
        /// ID of the status effect
        effect_id: u32,
        /// Duration in turns (None = permanent)
        duration: Option<u32>,
    },

    /// A status effect has been removed
    StatusEffectRemoved {
        /// Entity losing the status effect
        target: Entity,
        /// ID of the status effect
        effect_id: u32,
    },

    /// An entity's HP has changed
    HpChanged {
        /// Entity whose HP changed
        entity: Entity,
        /// Old HP value
        old_hp: i32,
        /// New HP value
        new_hp: i32,
        /// Maximum HP
        max_hp: i32,
    },

    /// An entity's MP has changed
    MpChanged {
        /// Entity whose MP changed
        entity: Entity,
        /// Old MP value
        old_mp: i32,
        /// New MP value
        new_mp: i32,
        /// Maximum MP
        max_mp: i32,
    },
}

/// Rewards granted after battle
#[derive(Debug, Clone, Default)]
pub struct BattleRewards {
    /// Experience points gained
    pub xp: u32,
    /// Gold gained
    pub gold: u32,
    /// Item IDs dropped
    pub items: Vec<u32>,
}

impl BattleRewards {
    /// Create a new empty rewards struct
    pub fn new() -> Self {
        Self::default()
    }

    /// Add XP to rewards
    pub fn with_xp(mut self, xp: u32) -> Self {
        self.xp = xp;
        self
    }

    /// Add gold to rewards
    pub fn with_gold(mut self, gold: u32) -> Self {
        self.gold = gold;
        self
    }

    /// Add an item to rewards
    pub fn with_item(mut self, item_id: u32) -> Self {
        self.items.push(item_id);
        self
    }
}

impl Event for BattleEvent {
    fn event_type(&self) -> EventType {
        EventType::Battle
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
}

/// Helper type alias for battle event handler results
pub type BattleResult<T> = Result<T, BattleError>;

/// Errors that can occur during battle operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum BattleError {
    /// Entity not found in battle
    #[error("Entity not found in battle: {0:?}")]
    EntityNotFound(Entity),

    /// Invalid action for current state
    #[error("Invalid action for current battle state")]
    InvalidAction,

    /// Not enough resources (MP, items, etc.)
    #[error("Not enough resources: {resource}")]
    InsufficientResources { resource: String },

    /// Target is invalid
    #[error("Invalid target: {reason}")]
    InvalidTarget { reason: String },

    /// Battle has already ended
    #[error("Battle has already ended")]
    BattleAlreadyEnded,

    /// Cannot flee from this battle
    #[error("Cannot flee from this battle")]
    CannotFlee,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battle_event_type() {
        let event = BattleEvent::BattleStarted { encounter_id: 1 };
        assert_eq!(event.event_type(), EventType::Battle);
    }

    #[test]
    fn test_battle_event_downcast() {
        use crate::events::downcast_event;

        let event = BattleEvent::BattleStarted { encounter_id: 42 };
        let event_ref: &dyn Event = &event;

        let downcast = downcast_event::<BattleEvent>(event_ref);
        assert!(downcast.is_some());

        if let Some(BattleEvent::BattleStarted { encounter_id }) = downcast {
            assert_eq!(*encounter_id, 42);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_battle_rewards_builder() {
        let rewards = BattleRewards::new()
            .with_xp(100)
            .with_gold(50)
            .with_item(1)
            .with_item(2);

        assert_eq!(rewards.xp, 100);
        assert_eq!(rewards.gold, 50);
        assert_eq!(rewards.items, vec![1, 2]);
    }

    #[test]
    fn test_battle_event_variants() {
        let entity = Entity::DANGLING;

        // Test all variants can be created
        let _ = BattleEvent::BattleStarted { encounter_id: 1 };
        let _ = BattleEvent::TurnStarted { entity };
        let _ = BattleEvent::TurnEnded { entity };
        let _ = BattleEvent::DamageDealt {
            attacker: entity,
            defender: entity,
            amount: 10,
            critical: false,
        };
        let _ = BattleEvent::HealingApplied {
            source: entity,
            target: entity,
            amount: 20,
        };
        let _ = BattleEvent::EnemyDefeated {
            entity,
            xp_gained: 50,
        };
        let _ = BattleEvent::ItemUsed {
            user: entity,
            item_id: 1,
            target: Some(entity),
        };
        let _ = BattleEvent::SkillUsed {
            user: entity,
            skill_id: 1,
            target: None,
        };
        let _ = BattleEvent::BattleEnded {
            victory: true,
            rewards: BattleRewards::new(),
        };
        let _ = BattleEvent::FleeAttempt {
            entity,
            success: true,
        };
        let _ = BattleEvent::StatusEffectApplied {
            target: entity,
            effect_id: 1,
            duration: Some(3),
        };
        let _ = BattleEvent::StatusEffectRemoved {
            target: entity,
            effect_id: 1,
        };
        let _ = BattleEvent::HpChanged {
            entity,
            old_hp: 100,
            new_hp: 80,
            max_hp: 100,
        };
        let _ = BattleEvent::MpChanged {
            entity,
            old_mp: 50,
            new_mp: 40,
            max_mp: 50,
        };
    }

    #[test]
    fn test_battle_error_display() {
        let err = BattleError::EntityNotFound(Entity::DANGLING);
        assert!(err.to_string().contains("Entity not found"));

        let err = BattleError::InvalidAction;
        assert!(err.to_string().contains("Invalid action"));

        let err = BattleError::InsufficientResources {
            resource: "MP".to_string(),
        };
        assert!(err.to_string().contains("Not enough resources"));

        let err = BattleError::InvalidTarget {
            reason: "dead".to_string(),
        };
        assert!(err.to_string().contains("Invalid target"));

        let err = BattleError::BattleAlreadyEnded;
        assert!(err.to_string().contains("already ended"));

        let err = BattleError::CannotFlee;
        assert!(err.to_string().contains("Cannot flee"));
    }
}
