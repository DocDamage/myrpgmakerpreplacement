//! Auto-Documentation Generator
//!
//! AI-powered documentation generation for RPG projects.
//! Generates World Bibles, Character Profiles, Quest Logs, and Store Descriptions.

pub mod generator;
pub mod templates;
pub mod exporters;

pub use generator::DocGenerator;
pub use templates::DocumentTemplate;
pub use exporters::{export_markdown, export_pdf, export_wiki};
