//! Document Exporters
//!
//! Export documentation to various formats: Markdown, PDF, Wiki

use super::generator::{CharacterProfile, QuestLog, WorldBible};

/// Export to Markdown
pub fn export_markdown(world_bible: &WorldBible, characters: &[CharacterProfile]) -> String {
    let mut output = String::new();

    // World Bible
    output.push_str(&format!("# {} - World Bible\n\n", world_bible.title));
    output.push_str(&format!(
        "> Generated: {}\n\n",
        world_bible.generated_at.format("%Y-%m-%d %H:%M")
    ));

    // Lore
    output.push_str("## Lore & Mythology\n\n");
    output.push_str(&format!(
        "### Creation Myth\n{}\n\n",
        world_bible.lore.creation_myth
    ));
    output.push_str(&format!(
        "### Historical Events\n{}\n\n",
        world_bible.lore.history.join("\n")
    ));
    output.push_str(&format!(
        "### Cultural Practices\n{}\n\n",
        world_bible.lore.culture
    ));

    // Timeline
    output.push_str("## Timeline\n\n");
    for event in &world_bible.timeline {
        output.push_str(&format!(
            "- **{}**: {}\n",
            event.year, event.description
        ));
    }
    output.push('\n');

    // Geography
    output.push_str("## Geography\n\n");
    output.push_str(&world_bible.geography.description);
    output.push('\n');

    // Factions
    output.push_str("## Factions\n\n");
    for faction in &world_bible.factions {
        output.push_str(&format!("### {}\n{}\n\n", faction.name, faction.description));
    }

    // Characters
    output.push_str("## Notable Characters\n\n");
    for character in characters {
        output.push_str(&format!("### {}\n", character.name));
        output.push_str(&format!("{}\n\n", character.physical_description));
        output.push_str(&format!(
            "**Personality:** {}\n\n",
            character.personality.join(", ")
        ));
        output.push_str(&format!("**Background:** {}\n\n", character.background));
    }

    output
}

/// Export quest log to Markdown
pub fn export_quest_log(quest_log: &QuestLog) -> String {
    let mut output = String::new();

    output.push_str("# Quest Log\n\n");
    output.push_str(&format!(
        "> Generated: {}\n\n",
        quest_log.generated_at.format("%Y-%m-%d %H:%M")
    ));

    // Story Arcs
    output.push_str("## Story Arcs\n\n");
    for arc in &quest_log.story_arcs {
        output.push_str(&format!("### {}\n", arc.name));
        output.push_str(&format!("{}\n\n", arc.description));
        output.push_str("**Quests in Arc:**\n");
        for quest_name in &arc.quest_names {
            output.push_str(&format!("- {}\n", quest_name));
        }
        output.push('\n');
    }

    // All Quests
    output.push_str("## All Quests\n\n");
    for quest in &quest_log.quests {
        output.push_str(&format!("### {}\n", quest.name));
        output.push_str(&format!("{}\n\n", quest.description));
        output.push_str(&format!("- **Type:** {:?}\n", quest.quest_type));
        output.push_str(&format!("- **Difficulty:** {}\n", quest.difficulty));
        if let Some(ref giver) = quest.giver_name {
            output.push_str(&format!("- **Quest Giver:** {}\n", giver));
        }
        output.push('\n');
    }

    output
}

/// Export to PDF (via markdown -> HTML)
pub fn export_pdf(world_bible: &WorldBible, characters: &[CharacterProfile]) -> Vec<u8> {
    let markdown = export_markdown(world_bible, characters);

    // Convert markdown to HTML
    let html = markdown_to_html(&markdown);

    // Wrap in PDF-friendly HTML
    let pdf_html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {{ font-family: Georgia, serif; line-height: 1.6; max-width: 800px; margin: 0 auto; padding: 20px; }}
        h1 {{ color: #333; border-bottom: 2px solid #333; }}
        h2 {{ color: #555; margin-top: 30px; }}
        h3 {{ color: #777; }}
        blockquote {{ border-left: 3px solid #ccc; margin: 0; padding-left: 20px; color: #666; }}
        .page-break {{ page-break-after: always; }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
        html
    );

    pdf_html.into_bytes() // In production, use a PDF crate like printpdf
}

/// Export to Wiki format (MediaWiki)
pub fn export_wiki(world_bible: &WorldBible, characters: &[CharacterProfile]) -> String {
    let mut output = String::new();

    output.push_str(&format!("= {} =\n\n", world_bible.title));

    // Main page
    output.push_str("== Lore ==\n\n");
    output.push_str(&world_bible.lore.creation_myth);
    output.push_str("\n\n");

    output.push_str("== Historical Events ==\n\n");
    for event in &world_bible.timeline {
        output.push_str(&format!("* '''{}''': {}\n", event.year, event.description));
    }
    output.push('\n');

    output.push_str("== Geography ==\n\n");
    output.push_str(&world_bible.geography.description);
    output.push_str("\n\n");

    output.push_str("== Factions ==\n\n");
    for faction in &world_bible.factions {
        output.push_str(&format!("=== {} ===\n", faction.name));
        output.push_str(&format!("{}\n\n", faction.description));
    }

    // Character pages
    output.push_str("== Characters ==\n\n");
    for character in characters {
        output.push_str(&format!("=== {} ===\n\n", character.name));
        output.push_str(&format!("{}\n\n", character.physical_description));
        output.push_str("==== Personality ====\n");
        for trait_ in &character.personality {
            output.push_str(&format!("* {}\n", trait_));
        }
        output.push('\n');
        output.push_str("==== Background ====\n");
        output.push_str(&character.background);
        output.push_str("\n\n");
    }

    output
}

/// Convert markdown to HTML (simple implementation)
#[allow(clippy::manual_strip)]
fn markdown_to_html(markdown: &str) -> String {
    // This is a simplified markdown-to-HTML conversion
    // In production, use pulldown-cmark or similar
    let mut html = String::new();
    let mut in_list = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("# ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h1>{}</h1>\n", &trimmed[2..]));
        } else if trimmed.starts_with("## ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h2>{}</h2>\n", &trimmed[3..]));
        } else if trimmed.starts_with("### ") {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str(&format!("<h3>{}</h3>\n", &trimmed[4..]));
        } else if trimmed.starts_with("- ") {
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>\n", &trimmed[2..]));
        } else if trimmed.is_empty() {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            html.push_str("<p></p>\n");
        } else if trimmed.starts_with("> ") {
            html.push_str(&format!("<blockquote>{}</blockquote>\n", &trimmed[2..]));
        } else if trimmed.starts_with("**") && trimmed.ends_with("**") {
            let content = &trimmed[2..trimmed.len() - 2];
            html.push_str(&format!("<p><strong>{}</strong></p>\n", content));
        } else {
            html.push_str(&format!("<p>{}</p>\n", trimmed));
        }
    }

    if in_list {
        html.push_str("</ul>\n");
    }

    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::generator::{FactionProfile, GeographySection, HistoricalEvent, LoreSection};
    use chrono::Utc;

    fn create_test_world_bible() -> WorldBible {
        WorldBible {
            title: "Test World".to_string(),
            lore: LoreSection {
                creation_myth: "In the beginning...".to_string(),
                history: vec!["Event 1".to_string(), "Event 2".to_string()],
                culture: "Rich and diverse".to_string(),
            },
            timeline: vec![
                HistoricalEvent {
                    year: "Year 1".to_string(),
                    description: "The beginning".to_string(),
                },
            ],
            geography: GeographySection {
                description: "A vast world".to_string(),
                regions: vec![],
            },
            factions: vec![FactionProfile {
                name: "The Alliance".to_string(),
                description: "Good guys".to_string(),
                goals: vec![],
                relationships: vec![],
            }],
            generated_at: Utc::now(),
        }
    }

    fn create_test_characters() -> Vec<CharacterProfile> {
        vec![CharacterProfile {
            name: "Hero".to_string(),
            physical_description: "Tall and strong".to_string(),
            personality: vec!["Brave".to_string(), "Kind".to_string()],
            background: "From a small village".to_string(),
            motivations: "Save the world".to_string(),
            relationships: vec!["Friend of All".to_string()],
            portrait_prompt: "fantasy hero portrait".to_string(),
        }]
    }

    #[test]
    fn test_export_markdown_contains_title() {
        let bible = create_test_world_bible();
        let characters = create_test_characters();
        let markdown = export_markdown(&bible, &characters);

        assert!(markdown.contains("# Test World - World Bible"));
        assert!(markdown.contains("## Lore & Mythology"));
        assert!(markdown.contains("### Hero"));
    }

    #[test]
    fn test_export_wiki_format() {
        let bible = create_test_world_bible();
        let characters = create_test_characters();
        let wiki = export_wiki(&bible, &characters);

        assert!(wiki.contains("= Test World ="));
        assert!(wiki.contains("== Lore =="));
        assert!(wiki.contains("=== Hero ==="));
    }

    #[test]
    fn test_markdown_to_html_conversion() {
        let markdown = "# Heading\n\nSome text\n\n- Item 1\n- Item 2";
        let html = markdown_to_html(markdown);

        assert!(html.contains("<h1>Heading</h1>"));
        assert!(html.contains("<p>Some text</p>"));
        assert!(html.contains("<li>Item 1</li>"));
    }
}
