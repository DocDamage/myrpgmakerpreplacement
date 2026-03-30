//! AI Director Control Panel
//!
//! Editor UI for managing the AI Game Director system.
//! Shows quest proposals, active quests, tension graphs, and settings.

use dde_ai::director::{
    ActiveQuest, DirectorConfig, DirectorStats, DirectorSystem, QuestProposal, QuestStage, TensionCurve,
};

/// Director panel UI state
pub struct DirectorPanel {
    /// Whether panel is visible
    visible: bool,
    /// Selected tab
    selected_tab: DirectorTab,
    /// Selected proposal index (for UI highlighting)
    selected_proposal: Option<usize>,
    /// Selected active quest index
    selected_quest: Option<usize>,
    /// Whether to auto-generate quests
    auto_generate: bool,
    /// Tension curve visualization data
    tension_graph: Vec<f32>,
    /// Status message
    status_message: Option<String>,
    /// Status timeout
    status_timeout: f32,
    /// Pending configuration changes
    pending_config: DirectorConfig,
}

/// Director panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectorTab {
    Proposals,
    ActiveQuests,
    History,
    Settings,
    Analytics,
}

impl DirectorPanel {
    /// Create a new director panel
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_tab: DirectorTab::Proposals,
            selected_proposal: None,
            selected_quest: None,
            auto_generate: true,
            tension_graph: Vec::with_capacity(100),
            status_message: None,
            status_timeout: 0.0,
            pending_config: DirectorConfig::default(),
        }
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update panel state
    pub fn update(&mut self, dt: f32) {
        // Update status timeout
        if self.status_timeout > 0.0 {
            self.status_timeout -= dt;
            if self.status_timeout <= 0.0 {
                self.status_message = None;
            }
        }

        // Update tension graph (keep last 100 samples)
        if self.tension_graph.len() >= 100 {
            self.tension_graph.remove(0);
        }
    }

    /// Draw the director panel UI
    pub fn draw(&mut self, ctx: &egui::Context, director: Option<&mut DirectorSystem>) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("🎮 AI Director")
            .open(&mut visible)
            .resizable(true)
            .default_size([700.0, 500.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, director);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui, mut director: Option<&mut DirectorSystem>) {
        // Header with enable/disable toggle
        ui.horizontal(|ui| {
            ui.heading("AI Game Director");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(dir) = director.as_mut() {
                    if ui
                        .checkbox(&mut dir.enabled, "Enabled")
                        .changed()
                    {
                        if dir.enabled {
                            self.show_status("Director enabled", false);
                        } else {
                            self.show_status("Director disabled", false);
                        }
                    }
                }
            });
        });

        ui.separator();

        // Stats bar
        if let Some(dir) = director.as_ref() {
            self.draw_stats_bar(ui, &dir.stats());
        }

        ui.separator();

        // Tab bar
        ui.horizontal(|ui| {
            self.tab_button(ui, "📜 Proposals", DirectorTab::Proposals);
            self.tab_button(ui, "⚔️ Active", DirectorTab::ActiveQuests);
            self.tab_button(ui, "📚 History", DirectorTab::History);
            self.tab_button(ui, "⚙️ Settings", DirectorTab::Settings);
            self.tab_button(ui, "📊 Analytics", DirectorTab::Analytics);
        });

        ui.separator();

        // Tab content
        match self.selected_tab {
            DirectorTab::Proposals => {
                if let Some(dir) = director {
                    self.draw_proposals_tab(ui, dir);
                }
            }
            DirectorTab::ActiveQuests => {
                if let Some(dir) = director {
                    self.draw_active_quests_tab(ui, dir);
                }
            }
            DirectorTab::History => {
                if let Some(dir) = director {
                    self.draw_history_tab(ui, dir);
                }
            }
            DirectorTab::Settings => {
                self.draw_settings_tab(ui, director);
            }
            DirectorTab::Analytics => {
                if let Some(dir) = director {
                    self.draw_analytics_tab(ui, dir);
                }
            }
        }

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.separator();
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: DirectorTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, label).clicked() {
            self.selected_tab = tab;
        }
    }

    /// Draw stats bar
    fn draw_stats_bar(&mut self, ui: &mut egui::Ui, stats: &DirectorStats) {
        ui.horizontal(|ui| {
            ui.label(format!("Active: {}", stats.active_quests));
            ui.label(format!("Completed: {}", stats.completed_quests));
            ui.label(format!("Failed: {}", stats.failed_quests));
            ui.label(format!("Tension: {:.0}%", stats.current_tension * 100.0));

            // Tension indicator with color
            let tension_color = if stats.current_tension < 0.3 {
                egui::Color32::GREEN
            } else if stats.current_tension < 0.7 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            };

            ui.label("| Tension:");
            ui.colored_label(tension_color, format!("{:.0}%", stats.current_tension * 100.0));

            ui.label(format!(
                "| Last Gen: {:.0}s ago",
                stats.time_since_last_generation
            ));
        });
    }

    /// Draw proposals tab
    fn draw_proposals_tab(&mut self, ui: &mut egui::Ui, director: &mut DirectorSystem) {
        let proposals: Vec<_> = director.get_proposals().iter().cloned().collect();

        if proposals.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("No quest proposals available").weak());
                ui.label("The director will generate proposals based on game state.");
                ui.add_space(20.0);

                if ui.button("🎲 Force Generate").clicked() {
                    self.show_status("Queued quest generation...", false);
                    // Note: Actual generation would need async handling
                }
            });
            return;
        }

        ui.label(format!("{} Quest Proposals", proposals.len()));
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, proposal) in proposals.iter().enumerate() {
                self.draw_proposal_card(ui, idx, proposal, director);
            }
        });
    }

    /// Draw a quest proposal card
    fn draw_proposal_card(
        &mut self,
        ui: &mut egui::Ui,
        idx: usize,
        proposal: &QuestProposal,
        director: &mut DirectorSystem,
    ) {
        let is_selected = self.selected_proposal == Some(idx);

        egui::Frame::group(ui.style())
            .fill(if is_selected {
                ui.visuals().widgets.active.bg_fill
            } else {
                ui.visuals().panel_fill
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Header row
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(proposal.quest_type.icon()).size(20.0),
                    );
                    ui.heading(&proposal.title);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Difficulty badge
                        let diff_color = proposal.difficulty_estimate.color();
                        ui.colored_label(
                            egui::Color32::from_rgb(diff_color[0], diff_color[1], diff_color[2]),
                            proposal.difficulty_estimate.name(),
                        );
                    });
                });

                ui.label(&proposal.description);

                ui.separator();

                // Details row
                ui.horizontal(|ui| {
                    ui.label(format!("Type: {}", proposal.quest_type.name()));
                    ui.label(format!("Location: {}", proposal.location_hint));

                    // Confidence indicator
                    let conf_color = if proposal.confidence_score > 0.8 {
                        egui::Color32::GREEN
                    } else if proposal.confidence_score > 0.5 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.label("Confidence:");
                    ui.colored_label(conf_color, format!("{:.0}%", proposal.confidence_score * 100.0));
                });

                // NPCs
                if !proposal.involved_npcs.is_empty() && proposal.involved_npcs[0] != "new" {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("NPCs:");
                        for npc in &proposal.involved_npcs {
                            ui.label(egui::RichText::new(npc).italics());
                        }
                    });
                }

                // Rewards
                ui.horizontal_wrapped(|ui| {
                    ui.label("Rewards:");
                    for reward in &proposal.suggested_rewards {
                        ui.label(format!("{:?} x{}", reward.reward_type, reward.amount));
                    }
                });

                ui.separator();

                // Actions
                ui.horizontal(|ui| {
                    if ui.button("✓ Accept").clicked() {
                        if let Some(quest) = director.accept_proposal(idx) {
                            self.show_status(&format!("Quest accepted: {}", quest.title), false);
                        }
                    }

                    if ui.button("🔄 Regenerate").clicked() {
                        director.reject_proposal(idx);
                        self.show_status("Proposal rejected, will regenerate", false);
                    }

                    if ui.button("✗ Reject").clicked() {
                        director.reject_proposal(idx);
                        self.show_status("Proposal rejected", false);
                    }
                });
            });

        ui.add_space(8.0);
    }

    /// Draw active quests tab
    fn draw_active_quests_tab(&mut self, ui: &mut egui::Ui, director: &mut DirectorSystem) {
        let quests: Vec<_> = director.get_active_quests().iter().cloned().collect();

        if quests.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("No active quests").weak());
                ui.label("Accept quest proposals to see them here.");
            });
            return;
        }

        ui.label(format!("{} Active Quests", quests.len()));
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, quest) in quests.iter().enumerate() {
                self.draw_active_quest_card(ui, idx, quest, director);
            }
        });
    }

    /// Draw active quest card
    fn draw_active_quest_card(
        &mut self,
        ui: &mut egui::Ui,
        idx: usize,
        quest: &ActiveQuest,
        director: &mut DirectorSystem,
    ) {
        let stage_color = quest.stage.color();

        egui::Frame::group(ui.style())
            .fill(ui.visuals().panel_fill)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(quest.quest_type.icon()).size(20.0));
                    ui.heading(&quest.title);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Stage badge
                        ui.colored_label(
                            egui::Color32::from_rgb(stage_color[0], stage_color[1], stage_color[2]),
                            quest.stage.name(),
                        );
                    });
                });

                ui.label(&quest.description);

                ui.separator();

                // Progress bar
                let progress = quest.completion_percentage();
                let progress_text = format!("{:.0}%", progress * 100.0);
                ui.add(
                    egui::ProgressBar::new(progress)
                        .text(progress_text)
                        .desired_width(ui.available_width()),
                );

                // Objectives
                ui.collapsing("Objectives", |ui| {
                    for obj in &quest.objectives {
                        let obj_progress = obj.progress_percentage();
                        ui.horizontal(|ui| {
                            if obj.completed {
                                ui.label("✓");
                            } else {
                                ui.label("○");
                            }
                            ui.label(&obj.description);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("{}/{}", obj.current, obj.required));
                            });
                        });
                    }
                });

                // Actions
                ui.horizontal(|ui| {
                    if quest.stage == QuestStage::ReadyForTurnIn {
                        if ui.button("✓ Complete").clicked() {
                            director.complete_quest(quest.id);
                            self.show_status(&format!("Completed: {}", quest.title), false);
                        }
                    }

                    if ui.button("✗ Abandon").clicked() {
                        director.quest_pool.abandon_quest(quest.id);
                        self.show_status(&format!("Abandoned: {}", quest.title), false);
                    }
                });
            });

        ui.add_space(8.0);
    }

    /// Draw history tab
    fn draw_history_tab(&mut self, ui: &mut egui::Ui, director: &DirectorSystem) {
        let history: Vec<_> = director.get_quest_history().iter().cloned().collect();

        if history.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("No quest history").weak());
            });
            return;
        }

        ui.label(format!("{} Quests in History", history.len()));
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in history.iter().rev() {
                let outcome_color = match entry.outcome {
                    dde_ai::director::QuestOutcome::Completed => egui::Color32::GREEN,
                    dde_ai::director::QuestOutcome::Failed { .. } => egui::Color32::RED,
                    dde_ai::director::QuestOutcome::Abandoned => egui::Color32::YELLOW,
                };

                ui.horizontal(|ui| {
                    ui.label(entry.quest_type.icon());
                    ui.label(&entry.title);
                    ui.colored_label(outcome_color, entry.outcome.name());
                });
            }
        });
    }

    /// Draw settings tab
    fn draw_settings_tab(&mut self, ui: &mut egui::Ui, director: Option<&mut DirectorSystem>) {
        ui.heading("Director Settings");
        ui.separator();

        // Auto-generate toggle
        ui.checkbox(&mut self.auto_generate, "Auto-generate quests");
        ui.label("Automatically generate quest proposals based on game state.");

        ui.separator();

        // Quest generation settings
        ui.heading("Generation Settings");
        ui.horizontal(|ui| {
            ui.label("Quest Cooldown:");
            ui.add(
                egui::DragValue::new(&mut self.pending_config.quest_cooldown)
                    .speed(10.0)
                    .suffix("s"),
            );
        });
        ui.label("Minimum time between quest generations.");

        ui.horizontal(|ui| {
            ui.label("Max Active Quests:");
            ui.add(
                egui::DragValue::new(&mut self.pending_config.max_active_quests)
                    .speed(1)
                    .range(1..=10),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Max Proposals:");
            ui.add(
                egui::DragValue::new(&mut self.pending_config.max_proposals)
                    .speed(1)
                    .range(1..=10),
            );
        });

        ui.separator();

        // Pacing settings
        ui.heading("Pacing Settings");

        egui::ComboBox::from_label("Tension Curve")
            .selected_text(self.pending_config.pacing.tension_curve.name())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.pending_config.pacing.tension_curve,
                    TensionCurve::Flat,
                    TensionCurve::Flat.name(),
                );
                ui.selectable_value(
                    &mut self.pending_config.pacing.tension_curve,
                    TensionCurve::Wave,
                    TensionCurve::Wave.name(),
                );
                ui.selectable_value(
                    &mut self.pending_config.pacing.tension_curve,
                    TensionCurve::Sawtooth,
                    TensionCurve::Sawtooth.name(),
                );
                ui.selectable_value(
                    &mut self.pending_config.pacing.tension_curve,
                    TensionCurve::Escalating,
                    TensionCurve::Escalating.name(),
                );
            });

        ui.label(self.pending_config.pacing.tension_curve.description());

        ui.horizontal(|ui| {
            ui.label("Quiet Time Threshold:");
            ui.add(
                egui::DragValue::new(&mut self.pending_config.pacing.quiet_time_threshold)
                    .speed(1.0)
                    .suffix("s"),
            );
        });

        ui.separator();

        // AI settings
        ui.heading("AI Settings");
        ui.checkbox(&mut self.pending_config.use_llm, "Use LLM for generation");
        ui.label("Use AI models for quest generation. Disable to use templates only.");

        ui.separator();

        // Apply button
        if ui.button("💾 Apply Settings").clicked() {
            if let Some(dir) = director {
                dir.config = self.pending_config.clone();
                dir.pacing = dde_ai::director::PacingController::with_config(
                    self.pending_config.pacing.clone(),
                );
                self.show_status("Settings applied", false);
            }
        }

        if ui.button("↩️ Reset to Defaults").clicked() {
            self.pending_config = DirectorConfig::default();
            self.show_status("Settings reset", false);
        }
    }

    /// Draw analytics tab
    fn draw_analytics_tab(&mut self, ui: &mut egui::Ui, director: &DirectorSystem) {
        let stats = director.stats();

        ui.heading("Director Analytics");
        ui.separator();

        // Stats grid
        egui::Grid::new("director_stats_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Total Proposals Generated:");
                ui.label(stats.total_proposals.to_string());
                ui.end_row();

                ui.label("Active Quests:");
                ui.label(stats.active_quests.to_string());
                ui.end_row();

                ui.label("Completed Quests:");
                ui.label(stats.completed_quests.to_string());
                ui.end_row();

                ui.label("Failed Quests:");
                ui.label(stats.failed_quests.to_string());
                ui.end_row();

                ui.label("Current Tension:");
                ui.label(format!("{:.1}%", stats.current_tension * 100.0));
                ui.end_row();

                ui.label("Time Since Last Gen:");
                ui.label(format!("{:.0}s", stats.time_since_last_generation));
                ui.end_row();
            });

        ui.separator();

        // Tension graph
        ui.heading("Tension History");
        self.draw_tension_graph(ui);
    }

    /// Draw tension visualization
    fn draw_tension_graph(&mut self, ui: &mut egui::Ui) {
        // Simple line graph
        let available_width = ui.available_width();
        let height = 100.0;

        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(available_width, height),
            egui::Sense::hover(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

            // Draw grid lines
            for i in 0..=4 {
                let y = rect.top() + rect.height() * (i as f32 / 4.0);
                painter.line_segment(
                    [
                        egui::pos2(rect.left(), y),
                        egui::pos2(rect.right(), y),
                    ],
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
            }

            // Draw tension line
            if self.tension_graph.len() >= 2 {
                let points: Vec<_> = self
                    .tension_graph
                    .iter()
                    .enumerate()
                    .map(|(i, &tension)| {
                        let x = rect.left()
                            + (i as f32 / (self.tension_graph.len() - 1) as f32) * rect.width();
                        let y = rect.bottom() - tension * rect.height();
                        egui::pos2(x, y)
                    })
                    .collect();

                // Draw line segments between points
                for i in 0..points.len().saturating_sub(1) {
                    painter.line_segment(
                        [points[i], points[i + 1]],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
                    );
                }
            }

            // Labels
            painter.text(
                rect.left_top() + egui::vec2(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                "High",
                egui::FontId::default(),
                ui.visuals().text_color(),
            );
            painter.text(
                rect.left_bottom() + egui::vec2(4.0, -4.0),
                egui::Align2::LEFT_BOTTOM,
                "Low",
                egui::FontId::default(),
                ui.visuals().text_color(),
            );
        }
    }

    /// Show a status message
    fn show_status(&mut self, message: &str, _is_error: bool) {
        self.status_message = Some(message.to_string());
        self.status_timeout = 5.0;
    }
}

impl Default for DirectorPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = DirectorPanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.selected_tab, DirectorTab::Proposals);
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = DirectorPanel::new();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_status_message() {
        let mut panel = DirectorPanel::new();
        panel.show_status("Test message", false);
        assert!(panel.status_message.is_some());
    }

    #[test]
    fn test_panel_update() {
        let mut panel = DirectorPanel::new();
        panel.show_status("Test", false);
        assert!(panel.status_message.is_some());

        // Update with time passage (less than 5 second timeout)
        panel.update(2.0);
        // Status should still be there
        assert!(panel.status_message.is_some());

        // Update past the timeout
        panel.update(5.0);
        // Status should be cleared
        assert!(panel.status_message.is_none());
    }
}
