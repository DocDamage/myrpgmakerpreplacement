//! Auto-Documentation Generator
//!
//! AI-powered documentation generation for RPG projects.
//! Generates World Bibles, Character Profiles, Quest Logs, and Store Descriptions.

pub mod exporters;
pub mod generator;
pub mod templates;

pub use exporters::{export_markdown, export_pdf, export_wiki};
pub use generator::DocGenerator;
pub use templates::DocumentTemplate;
