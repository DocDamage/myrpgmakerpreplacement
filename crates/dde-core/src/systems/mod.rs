//! ECS Systems
//!
//! Systems are functions that query and mutate components.
//! They have no state of their own.

pub mod animation;
pub mod bark;
pub mod dialogue;
pub mod input;
pub mod movement;
pub mod player;
pub mod simulation;

pub use bark::{BarkCategory, BarkDisplay, BarkRequest, BarkSystem, NpcBark};
pub use dialogue::{DialogueManager, DialogueSession, DialogueTree, TypewriterText};
pub use input::{InputBindings, InputContext, InputSystem};
pub use movement::{MovementSystem, TileCollisionMap};
pub use player::{Player, PlayerController};
