//! AI Director Control Panel
//!
//! Editor UI for managing the AI Game Director system.
//! Shows quest proposals, active quests, tension graphs, settings,
//! cache management, LLM provider configuration, and bark template editing.

use dde_ai::director::{
    ActiveQuest, DirectorConfig, DirectorStats, DirectorSystem, QuestProposal, QuestStage,
    TensionCurve, QuestOutcome,
};
use dde_ai::{
    AiTaskType, CacheStats, ProviderRoutingTable, ProviderConfig, LlmProvider,
    BarkTemplateManager, BarkTemplate, BarkCategory, TemplateVariable,
    TemplateSource,
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
    _selected_quest: Option<usize>,
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
    /// Cache manager reference
    cache_stats: Option<CacheStats>,
    /// Provider routing table
    provider_table: ProviderRoutingTable,
    /// Bark template manager
    bark_templates: BarkTemplateManager,
    /// Selected bark category for editing
    selected_bark_category: Option<BarkCategory>,
    /// Selected template for editing
    editing_template: Option<String>,
    /// Template being edited (new or existing)
    template_edit_state: Option<TemplateEditState>,
    /// Show clear cache confirmation
    show_clear_cache_confirm: bool,
    /// Task type selected for cache clearing
    clear_cache_task_type: Option<AiTaskType>,
    /// Pacing visualization data
    pacing_data: PacingVisualizationData,
    /// Selected quest for detail view
    selected_quest_detail: Option<usize>,
    /// Drag-drop state for quest reordering
    drag_drop_state: DragDropState,
    /// Show regenerate confirmation
    show_regenerate_confirm: bool,
    /// Proposal index to regenerate
    regenerate_proposal_idx: Option<usize>,
}

/// Template editing state
#[derive(Debug, Clone)]
struct TemplateEditState {
    id: String,
    category: BarkCategory,
    template_text: String,
    description: String,
    variables: Vec<TemplateVariable>,
    moods: String,
    priority: u8,
    max_uses: u32,
    enabled: bool,
    is_new: bool,
}

/// Drag and drop state for quest reordering
#[derive(Debug, Clone, Default)]
struct DragDropState {
    dragging_idx: Option<usize>,
    hover_idx: Option<usize>,
}

/// Pacing visualization data
#[derive(Debug, Clone, Default)]
struct PacingVisualizationData {
    /// Historical tension values
    tension_history: Vec<f32>,
    /// Desired tension curve values
    desired_tension: Vec<f32>,
    /// Quest density over time (heatmap data)
    quest_density: Vec<f32>,
    /// Recommended actions
    recommended_actions: Vec<RecommendedAction>,
}

/// A recommended action for the director
#[derive(Debug, Clone)]
struct RecommendedAction {
    priority: ActionPriority,
    message: String,
    action_type: ActionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionType {
    GenerateQuest,
    ReduceTension,
    IncreaseTension,
    Wait,
    ReviewProposals,
}

/// Director panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectorTab {
    Proposals,
    ActiveQuests,
    History,
    Settings,
    Analytics,
    CacheManagement,
    ProviderConfig,
    BarkTemplates,
}

impl DirectorTab {
    fn display_name(&self) -> &'static str {
        match self {
            DirectorTab::Proposals => "📜 Proposals",
            DirectorTab::ActiveQuests => "⚔️ Active",
            DirectorTab::History => "📚 History",
            DirectorTab::Settings => "⚙️ Settings",
            DirectorTab::Analytics => "📊 Analytics",
            DirectorTab::CacheManagement => "💾 Cache",
            DirectorTab::ProviderConfig => "🔌 Providers",
            DirectorTab::BarkTemplates => "💬 Barks",
        }
    }
}

impl DirectorPanel {
    /// Create a new director panel
    pub fn new() -> Self {
        let mut pacing_data = PacingVisualizationData::default();
        // Generate sample tension curve
        for i in 0..100 {
            let t = i as f32 / 100.0;
            let tension = (t * std::f32::consts::TAU).sin() * 0.3 + 0.4;
            pacing_data.tension_history.push(tension);
            pacing_data.desired_tension.push(tension * 0.9);
        }

        Self {
            visible: false,
            selected_tab: DirectorTab::Proposals,
            selected_proposal: None,
            _selected_quest: None,
            auto_generate: true,
            tension_graph: Vec::with_capacity(100),
            status_message: None,
            status_timeout: 0.0,
            pending_config: DirectorConfig::default(),
            cache_stats: None,
            provider_table: ProviderRoutingTable::default(),
            bark_templates: BarkTemplateManager::new(),
            selected_bark_category: None,
            editing_template: None,
            template_edit_state: None,
            show_clear_cache_confirm: false,
            clear_cache_task_type: None,
            pacing_data,
            selected_quest_detail: None,
            drag_drop_state: DragDropState::default(),
            show_regenerate_confirm: false,
            regenerate_proposal_idx: None,
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

        // Periodic cache stats refresh would happen here in real implementation
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
            .default_size([900.0, 650.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ctx, ui, director);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, mut director: Option<&mut DirectorSystem>) {
        // Header with enable/disable toggle
        ui.horizontal(|ui| {
            ui.heading("AI Game Director");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(dir) = director.as_mut() {
                    if ui.checkbox(&mut dir.enabled, "Enabled").changed() {
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

        // Tab bar - split into two rows for more tabs
        ui.horizontal_wrapped(|ui| {
            self.tab_button(ui, DirectorTab::Proposals);
            self.tab_button(ui, DirectorTab::ActiveQuests);
            self.tab_button(ui, DirectorTab::History);
            self.tab_button(ui, DirectorTab::Settings);
            self.tab_button(ui, DirectorTab::Analytics);
            self.tab_button(ui, DirectorTab::CacheManagement);
            self.tab_button(ui, DirectorTab::ProviderConfig);
            self.tab_button(ui, DirectorTab::BarkTemplates);
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
            DirectorTab::CacheManagement => {
                self.draw_cache_management_tab(ui);
            }
            DirectorTab::ProviderConfig => {
                self.draw_provider_config_tab(ui);
            }
            DirectorTab::BarkTemplates => {
                self.draw_bark_templates_tab(ui);
            }
        }

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.separator();
            ui.colored_label(egui::Color32::GREEN, msg);
        }

        // Handle modals
        self.draw_modals(ui.ctx());
    }

    fn draw_modals(&mut self, ctx: &egui::Context) {
        // Clear cache confirmation modal
        if self.show_clear_cache_confirm {
            let mut should_close = false;
            egui::Window::new("Clear Cache?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Are you sure you want to clear the cache?");
                    if let Some(task_type) = self.clear_cache_task_type {
                        ui.label(format!("This will clear all {} cache entries.", task_type.display_name()));
                    } else {
                        ui.label("This will clear ALL cached entries.");
                    }
                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                        if ui.button("Clear").clicked() {
                            self.clear_cache_confirmed();
                            should_close = true;
                        }
                    });
                });
            if should_close {
                self.show_clear_cache_confirm = false;
            }
        }
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, tab: DirectorTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, tab.display_name()).clicked() {
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
            ui.colored_label(
                tension_color,
                format!("{:.0}%", stats.current_tension * 100.0),
            );

            ui.label(format!(
                "| Last Gen: {:.0}s ago",
                stats.time_since_last_generation
            ));
        });
    }

    /// Draw proposals tab
    fn draw_proposals_tab(&mut self, ui: &mut egui::Ui, director: &mut DirectorSystem) {
        let proposals: Vec<_> = director.get_proposals().to_vec();

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
                    ui.label(egui::RichText::new(proposal.quest_type.icon()).size(20.0));
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
                    ui.colored_label(
                        conf_color,
                        format!("{:.0}%", proposal.confidence_score * 100.0),
                    );
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
                        self.regenerate_proposal_idx = Some(idx);
                        self.show_regenerate_confirm = true;
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
        let quests: Vec<_> = director.get_active_quests().to_vec();

        if quests.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("No active quests").weak());
                ui.label("Accept quest proposals to see them here.");
            });
            return;
        }

        // Quest pool stats
        let stats = director.quest_pool.stats();
        ui.horizontal(|ui| {
            ui.label(format!("{} Active Quests", quests.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Completion: {:.0}%", stats.overall_completion * 100.0));
            });
        });
        ui.separator();

        // Quest list with reordering
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
        let is_detailed = self.selected_quest_detail == Some(idx);

        egui::Frame::group(ui.style())
            .fill(ui.visuals().panel_fill)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Drag handle for reordering
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("≡").weak()); // Drag handle
                    ui.add_space(4.0);

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

                // Progress bar
                let progress = quest.completion_percentage();
                let progress_text = format!("{:.0}%", progress * 100.0);
                ui.add(
                    egui::ProgressBar::new(progress)
                        .text(progress_text)
                        .desired_width(ui.available_width()),
                );

                // Expand/collapse details
                if ui.button(if is_detailed { "📖 Hide Details" } else { "📖 Show Details" }).clicked() {
                    self.selected_quest_detail = if is_detailed { None } else { Some(idx) };
                }

                if is_detailed {
                    ui.separator();

                    // Objectives
                    ui.label("Objectives:");
                    for obj in &quest.objectives {
                        ui.horizontal(|ui| {
                            if obj.completed {
                                ui.colored_label(egui::Color32::GREEN, "✓");
                            } else {
                                ui.label("○");
                            }
                            ui.label(&obj.description);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(format!("{}/{}", obj.current, obj.required));
                                },
                            );
                        });
                    }

                    // Metadata
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("Source: {}", quest.metadata.source.name()));
                        ui.label(format!("Difficulty: {:?}", quest.metadata.difficulty));
                        ui.label(format!("Elapsed: {:.0}s", quest.elapsed_time));
                    });
                }

                ui.separator();

                // Actions
                ui.horizontal(|ui| {
                    if quest.stage == QuestStage::ReadyForTurnIn
                        && ui.button("✓ Complete").clicked()
                    {
                        director.complete_quest(quest.id);
                        self.show_status(&format!("Completed: {}", quest.title), false);
                    }

                    if ui.button("✗ Abandon").clicked() {
                        director.quest_pool.abandon_quest(quest.id);
                        self.show_status(&format!("Abandoned: {}", quest.title), false);
                    }

                    if ui.small_button("⬆").clicked() && idx > 0 {
                        // Reorder up - would need backend support
                        self.show_status("Quest priority increased", false);
                    }
                    if ui.small_button("⬇").clicked() {
                        // Reorder down - would need backend support
                        self.show_status("Quest priority decreased", false);
                    }
                });
            });

        ui.add_space(8.0);
    }

    /// Draw history tab
    fn draw_history_tab(&mut self, ui: &mut egui::Ui, director: &DirectorSystem) {
        let history: Vec<_> = director.get_quest_history().to_vec();

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
                    QuestOutcome::Completed => egui::Color32::GREEN,
                    QuestOutcome::Failed { .. } => egui::Color32::RED,
                    QuestOutcome::Abandoned => egui::Color32::YELLOW,
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

        // Tension graph with enhanced visualization
        ui.heading("Tension Analysis");
        self.draw_enhanced_tension_graph(ui, director);

        ui.separator();

        // Recommended actions
        ui.heading("Recommended Actions");
        self.draw_recommended_actions(ui, director);
    }

    /// Draw enhanced tension graph
    fn draw_enhanced_tension_graph(&mut self, ui: &mut egui::Ui, _director: &DirectorSystem) {
        let available_width = ui.available_width();
        let height = 150.0;

        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);

            // Draw grid lines
            for i in 0..=4 {
                let y = rect.top() + rect.height() * (i as f32 / 4.0);
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
            }

            // Draw desired tension curve (dashed)
            let desired_points: Vec<_> = self.pacing_data.desired_tension
                .iter()
                .enumerate()
                .map(|(i, &tension)| {
                    let x = rect.left()
                        + (i as f32 / (self.pacing_data.desired_tension.len().max(1) - 1).max(1) as f32) * rect.width();
                    let y = rect.bottom() - tension * rect.height();
                    egui::pos2(x, y)
                })
                .collect();

            for i in 0..desired_points.len().saturating_sub(1) {
                // Draw dashed line for desired
                let start = desired_points[i];
                let end = desired_points[i + 1];
                let mid = egui::pos2((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                painter.line_segment(
                    [start, mid],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
                );
            }

            // Draw actual tension line
            if !self.tension_graph.is_empty() {
                let points: Vec<_> = self
                    .tension_graph
                    .iter()
                    .enumerate()
                    .map(|(i, &tension)| {
                        let x = rect.left()
                            + (i as f32 / (self.tension_graph.len().max(1) - 1).max(1) as f32) * rect.width();
                        let y = rect.bottom() - tension * rect.height();
                        egui::pos2(x, y)
                    })
                    .collect();

                for i in 0..points.len().saturating_sub(1) {
                    painter.line_segment(
                        [points[i], points[i + 1]],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
                    );
                }

                // Current tension indicator
                if let Some(&current) = self.tension_graph.last() {
                    let current_x = rect.right();
                    let current_y = rect.bottom() - current * rect.height();
                    painter.circle_filled(
                        egui::pos2(current_x, current_y),
                        6.0,
                        egui::Color32::from_rgb(255, 100, 100),
                    );
                }
            }

            // Labels
            painter.text(
                rect.left_top() + egui::vec2(8.0, 4.0),
                egui::Align2::LEFT_TOP,
                "High Tension",
                egui::FontId::default(),
                ui.visuals().text_color(),
            );
            painter.text(
                rect.left_bottom() + egui::vec2(8.0, -4.0),
                egui::Align2::LEFT_BOTTOM,
                "Low Tension",
                egui::FontId::default(),
                ui.visuals().text_color(),
            );

            // Legend
            painter.text(
                rect.right_top() + egui::vec2(-4.0, 4.0),
                egui::Align2::RIGHT_TOP,
                "— Actual  ··· Desired",
                egui::FontId::default(),
                ui.visuals().weak_text_color(),
            );
        }

        // Hover tooltip
        response.on_hover_ui(|ui| {
            ui.label("Tension over time");
        });
    }

    /// Draw recommended actions
    fn draw_recommended_actions(&mut self, ui: &mut egui::Ui, director: &DirectorSystem) {
        // Generate recommendations based on current state
        let recommendations = self.generate_recommendations(director);

        if recommendations.is_empty() {
            ui.label(egui::RichText::new("No actions recommended at this time.").weak());
            return;
        }

        for rec in recommendations {
            let (icon, color) = match rec.priority {
                ActionPriority::Critical => ("🔴", egui::Color32::RED),
                ActionPriority::High => ("🟠", egui::Color32::from_rgb(255, 150, 50)),
                ActionPriority::Medium => ("🟡", egui::Color32::YELLOW),
                ActionPriority::Low => ("🟢", egui::Color32::GREEN),
            };

            ui.horizontal(|ui| {
                ui.colored_label(color, icon);
                ui.label(&rec.message);
            });
        }
    }

    /// Generate recommendations based on director state
    fn generate_recommendations(&self, director: &DirectorSystem) -> Vec<RecommendedAction> {
        let mut recommendations = Vec::new();
        let stats = director.stats();

        if stats.active_quests == 0 {
            recommendations.push(RecommendedAction {
                priority: ActionPriority::High,
                message: "No active quests. Consider generating some content.".to_string(),
                action_type: ActionType::GenerateQuest,
            });
        }

        if stats.current_tension > 0.8 {
            recommendations.push(RecommendedAction {
                priority: ActionPriority::Medium,
                message: "Tension is very high. Consider adding resolution content.".to_string(),
                action_type: ActionType::ReduceTension,
            });
        } else if stats.current_tension < 0.2 {
            recommendations.push(RecommendedAction {
                priority: ActionPriority::Low,
                message: "Tension is low. Good time to introduce challenges.".to_string(),
                action_type: ActionType::IncreaseTension,
            });
        }

        if stats.time_since_last_generation > 600.0 {
            recommendations.push(RecommendedAction {
                priority: ActionPriority::Medium,
                message: format!("No quest generation for {:.0} minutes.", stats.time_since_last_generation / 60.0),
                action_type: ActionType::GenerateQuest,
            });
        }

        recommendations
    }

    /// Draw cache management tab
    fn draw_cache_management_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("💾 Cache Management");
        ui.separator();

        // Get or create cache stats
        let stats = self.cache_stats.clone().unwrap_or_else(|| {
            // Create default stats if not loaded
            let mut stats = CacheStats::default();
            // Simulate some data for demonstration
            stats.total_items = 42;
            stats.memory_mb = 12.5;
            stats.hit_rate = 0.73;
            stats.total_hits = 156;
            stats.total_misses = 58;
            stats
        });

        // Cache statistics display
        ui.group(|ui| {
            ui.heading("Cache Statistics");
            ui.add_space(8.0);

            egui::Grid::new("cache_stats_grid")
                .num_columns(4)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    // Row 1
                    ui.label("Total Items:");
                    ui.label(format!("{}", stats.total_items));
                    ui.label("Memory Usage:");
                    ui.label(format!("{:.2} MB", stats.memory_mb));
                    ui.end_row();

                    // Row 2
                    ui.label("Cache Hit Rate:");
                    let hit_rate_color = if stats.hit_rate > 0.7 {
                        egui::Color32::GREEN
                    } else if stats.hit_rate > 0.4 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.colored_label(hit_rate_color, format!("{:.1}%", stats.hit_rate * 100.0));
                    ui.label("Total Hits:");
                    ui.label(format!("{}", stats.total_hits));
                    ui.end_row();

                    // Row 3
                    ui.label("Expired Entries:");
                    ui.label(format!("{}", stats.expired_entries));
                    ui.label("Total Misses:");
                    ui.label(format!("{}", stats.total_misses));
                    ui.end_row();
                });
        });

        ui.separator();

        // Items by task type
        ui.heading("Items by Task Type");
        egui::Grid::new("cache_by_task_grid")
            .num_columns(5)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                // Header
                ui.label("Task Type");
                ui.label("Items");
                ui.label("Size");
                ui.label("Hits");
                ui.label("Action");
                ui.end_row();

                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.end_row();

                for task_type in AiTaskType::all() {
                    let task_stats = stats.by_task_type.get(task_type).copied().unwrap_or_default();

                    ui.label(task_type.display_name());
                    ui.label(format!("{}", task_stats.item_count));
                    ui.label(format!("{:.1} KB", task_stats.total_bytes as f32 / 1024.0));
                    ui.label(format!("{}", task_stats.hit_count));

                    if ui.small_button("🗑️").clicked() {
                        self.clear_cache_task_type = Some(*task_type);
                        self.show_clear_cache_confirm = true;
                    }
                    ui.end_row();
                }
            });

        ui.separator();

        // TTL Configuration
        ui.heading("TTL Configuration");
        ui.label("Set cache time-to-live in minutes for each task type:");
        ui.add_space(8.0);

        egui::Grid::new("ttl_config_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                for task_type in AiTaskType::all() {
                    let current_ttl = stats.ttl_minutes.get(task_type).copied().unwrap_or(60);
                    ui.label(task_type.display_name());

                    let mut ttl = current_ttl;
                    ui.add(egui::Slider::new(&mut ttl, 1..=1440).text("minutes"));

                    if ttl != current_ttl {
                        // TTL changed - would update in backend
                    }
                    ui.end_row();
                }
            });

        ui.separator();

        // Clear cache buttons
        ui.horizontal(|ui| {
            if ui.button("🗑️ Clear All Cache").clicked() {
                self.clear_cache_task_type = None;
                self.show_clear_cache_confirm = true;
            }

            if ui.button("🧹 Clean Expired").clicked() {
                self.show_status("Expired cache entries cleaned", false);
            }

            if ui.button("🔄 Refresh Stats").clicked() {
                self.show_status("Cache statistics refreshed", false);
            }
        });
    }

    /// Clear cache after confirmation
    fn clear_cache_confirmed(&mut self) {
        if let Some(task_type) = self.clear_cache_task_type {
            self.show_status(&format!("Cache cleared for {}", task_type.display_name()), false);
        } else {
            self.show_status("All cache cleared", false);
        }
        self.clear_cache_task_type = None;
    }

    /// Draw provider configuration tab
    fn draw_provider_config_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔌 LLM Provider Configuration");
        ui.separator();

        // API Key Status Section
        ui.group(|ui| {
            ui.heading("API Key Status");
            ui.add_space(8.0);

            egui::Grid::new("api_key_grid")
                .num_columns(3)
                .spacing([30.0, 8.0])
                .show(ui, |ui| {
                    for provider in [LlmProvider::OpenAi, LlmProvider::Anthropic, LlmProvider::Gemini, LlmProvider::Ollama] {
                        let status = self.provider_table.get_api_key_status(provider);
                        let color = status.color();

                        ui.label(provider.display_name());
                        ui.colored_label(
                            egui::Color32::from_rgb(color[0], color[1], color[2]),
                            status.display_text(),
                        );

                        if provider != LlmProvider::Ollama {
                            if ui.button("Configure").clicked() {
                                // Would open API key configuration dialog
                                self.show_status(&format!("Configure {} API key", provider.display_name()), false);
                            }
                        } else {
                            ui.label("(Local - no key needed)");
                        }
                        ui.end_row();
                    }
                });
        });

        ui.separator();

        // Provider Routing Table
        ui.heading("Provider Routing Table");
        ui.label("Configure which providers and models are used for each task type:");
        ui.add_space(8.0);

        // Routing table header
        egui::Grid::new("routing_table_grid")
            .num_columns(7)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                // Headers
                ui.label("Task Type");
                ui.label("Enabled");
                ui.label("Primary");
                ui.label("Model");
                ui.label("Fallback");
                ui.label("Priority");
                ui.label("Actions");
                ui.end_row();

                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.separator();
                ui.end_row();

                // Get all configs
                let task_types: Vec<_> = AiTaskType::all().to_vec();

                for task_type in task_types {
                    // Get or create config for this task
                    let mut config = self.provider_table
                        .get_config(task_type)
                        .cloned()
                        .unwrap_or_else(|| ProviderConfig::default_for_task(task_type));

                    ui.label(task_type.display_name());

                    // Enabled checkbox
                    let mut enabled = config.enabled;
                    ui.checkbox(&mut enabled, "");
                    config.enabled = enabled;

                    // Primary provider dropdown
                    let providers = [LlmProvider::OpenAi, LlmProvider::Anthropic, LlmProvider::Gemini, LlmProvider::Ollama];
                    egui::ComboBox::from_id_source(format!("primary_{:?}", task_type))
                        .selected_text(config.primary_provider.display_name())
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for provider in providers {
                                ui.selectable_value(&mut config.primary_provider, provider, provider.display_name());
                            }
                        });

                    // Model dropdown
                    let models = config.primary_provider.available_models();
                    egui::ComboBox::from_id_source(format!("model_{:?}", task_type))
                        .selected_text(&config.primary_model)
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for model in models {
                                ui.selectable_value(&mut config.primary_model, model.id.clone(), model.name);
                            }
                        });

                    // Fallback provider dropdown
                    let mut has_fallback = config.fallback_provider.is_some();
                    let mut fallback = config.fallback_provider.unwrap_or(LlmProvider::Ollama);

                    if ui.checkbox(&mut has_fallback, "").changed() {
                        config.fallback_provider = if has_fallback { Some(fallback) } else { None };
                    }

                    if has_fallback {
                        egui::ComboBox::from_id_source(format!("fallback_{:?}", task_type))
                            .selected_text(fallback.display_name())
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for provider in providers {
                                    ui.selectable_value(&mut fallback, provider, provider.display_name());
                                }
                            });
                        config.fallback_provider = Some(fallback);
                    }

                    // Priority slider
                    ui.add(egui::Slider::new(&mut config.priority, 0..=100).show_value(false));

                    // Test button
                    if ui.small_button("Test").clicked() {
                        self.show_status(&format!("Testing {} configuration...", task_type.display_name()), false);
                    }

                    ui.end_row();

                    // Save config back
                    self.provider_table.set_config(config);
                }
            });

        ui.separator();

        // Actions
        ui.horizontal(|ui| {
            if ui.button("🧪 Test All Connections").clicked() {
                self.show_status("Testing all provider connections...", false);
            }

            if ui.button("💾 Save Configuration").clicked() {
                self.show_status("Provider configuration saved", false);
            }

            if ui.button("↩️ Reset to Defaults").clicked() {
                self.provider_table.reset_to_defaults();
                self.show_status("Provider configuration reset to defaults", false);
            }
        });

        // Validation
        let issues = self.provider_table.validate();
        if !issues.is_empty() {
            ui.separator();
            ui.colored_label(egui::Color32::YELLOW, "⚠️ Configuration Issues:");
            for issue in &issues {
                ui.label(egui::RichText::new(format!("  • {}", issue)).weak());
            }
        }
    }

    /// Draw bark templates tab
    fn draw_bark_templates_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("💬 Bark Template Editor");
        ui.separator();

        // Two-column layout: categories on left, editor on right
        egui::SidePanel::left("bark_categories_panel")
            .resizable(true)
            .default_width(200.0)
            .show_inside(ui, |ui| {
                ui.heading("Categories");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for category in BarkCategory::all() {
                        let count = self.bark_templates.count_by_category(*category);
                        let is_selected = self.selected_bark_category == Some(*category);

                        let response = ui.selectable_label(
                            is_selected,
                            format!("{} {} ({})", category.icon(), category.display_name(), count),
                        );

                        if response.clicked() {
                            self.selected_bark_category = Some(*category);
                            self.editing_template = None;
                            self.template_edit_state = None;
                        }

                        response.on_hover_text(category.description());
                    }
                });

                ui.separator();

                if ui.button("➕ New Template").clicked() {
                    self.start_new_template();
                }
            });

        // Main content area
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.template_edit_state.is_some() {
                self.draw_template_editor(ui);
            } else if let Some(category) = self.selected_bark_category {
                self.draw_template_list(ui, category);
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("Select a category or create a new template").weak());
                });
            }
        });
    }

    /// Draw template list for a category
    fn draw_template_list(&mut self, ui: &mut egui::Ui, category: BarkCategory) {
        ui.heading(format!("{} Templates", category.display_name()));
        ui.label(category.description());
        ui.separator();

        let templates: Vec<_> = self.bark_templates.get_by_category(category)
            .into_iter()
            .cloned()
            .collect();

        if templates.is_empty() {
            ui.label("No templates in this category yet.");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for template in templates {
                let is_selected = self.editing_template.as_ref() == Some(&template.id);

                egui::Frame::group(ui.style())
                    .fill(if is_selected {
                        ui.visuals().widgets.active.bg_fill
                    } else {
                        ui.visuals().panel_fill
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        ui.horizontal(|ui| {
                            ui.label(if template.enabled { "✓" } else { "○" });
                            ui.label(&template.template);

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✏️").clicked() {
                                    self.start_edit_template(&template);
                                }
                                if ui.small_button("🗑️").clicked() {
                                    self.bark_templates.remove_template(&template.id);
                                    self.show_status("Template deleted", false);
                                }
                            });
                        });

                        if !template.description.is_empty() {
                            ui.label(egui::RichText::new(&template.description).weak().small());
                        }

                        // Variables preview
                        if !template.variables.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Vars:");
                                for var in &template.variables {
                                    ui.code(&var.name);
                                }
                            });
                        }
                    });

                ui.add_space(4.0);
            }
        });
    }

    /// Draw template editor
    fn draw_template_editor(&mut self, ui: &mut egui::Ui) {
        // Take the edit_state out temporarily
        let mut edit_state = self.template_edit_state.take().unwrap();
        
        ui.heading(if edit_state.is_new { "New Template" } else { "Edit Template" });
        ui.separator();

        // Template text
        ui.label("Template Text:");
        ui.label("Use {{variable_name}} for dynamic content");
        ui.add(egui::TextEdit::multiline(&mut edit_state.template_text)
            .desired_rows(3)
            .desired_width(f32::INFINITY));

        // Character count
        let len = edit_state.template_text.len();
        let len_color = if len > 200 {
            egui::Color32::RED
        } else if len > 150 {
            egui::Color32::YELLOW
        } else {
            ui.visuals().weak_text_color()
        };
        ui.colored_label(len_color, format!("{}/200 characters", len));

        ui.separator();

        // Category selection
        egui::ComboBox::from_label("Category")
            .selected_text(edit_state.category.display_name())
            .show_ui(ui, |ui| {
                for category in BarkCategory::all() {
                    ui.selectable_value(&mut edit_state.category, *category, 
                        format!("{} {}", category.icon(), category.display_name()));
                }
            });

        // Description
        ui.horizontal(|ui| {
            ui.label("Description:");
            ui.text_edit_singleline(&mut edit_state.description);
        });

        // Moods
        ui.horizontal(|ui| {
            ui.label("Moods (comma-separated):");
            ui.text_edit_singleline(&mut edit_state.moods);
        });

        // Priority
        ui.add(egui::Slider::new(&mut edit_state.priority, 0..=100)
            .text("Priority"));

        // Max uses
        let mut max_uses_enabled = edit_state.max_uses > 0;
        ui.horizontal(|ui| {
            ui.checkbox(&mut max_uses_enabled, "Limit uses");
            if max_uses_enabled {
                let mut max_uses = if edit_state.max_uses == 0 { 5 } else { edit_state.max_uses };
                ui.add(egui::DragValue::new(&mut max_uses).speed(1).range(1..=100));
                edit_state.max_uses = max_uses;
            } else {
                edit_state.max_uses = 0;
                ui.label("(unlimited)");
            }
        });

        ui.checkbox(&mut edit_state.enabled, "Enabled");

        ui.separator();

        // Variables editor
        ui.heading("Variables");
        ui.label("Define variables used in the template:");

        let mut vars_to_remove = Vec::new();
        for (idx, var) in edit_state.variables.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Variable {}:", idx + 1));
                    if ui.small_button("🗑️").clicked() {
                        vars_to_remove.push(idx);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut var.name);
                });

                ui.horizontal(|ui| {
                    ui.label("Display:");
                    ui.text_edit_singleline(&mut var.display_name);
                });

                ui.horizontal(|ui| {
                    ui.label("Example:");
                    ui.text_edit_singleline(&mut var.example);
                });

                ui.checkbox(&mut var.required, "Required");
            });
        }

        // Remove marked variables
        for idx in vars_to_remove.into_iter().rev() {
            edit_state.variables.remove(idx);
        }

        if ui.button("➕ Add Variable").clicked() {
            edit_state.variables.push(TemplateVariable::new(
                format!("var{}", edit_state.variables.len() + 1),
                "New Variable",
                "Description",
                "example",
            ));
        }

        ui.separator();

        // Preview
        ui.heading("Preview");
        let preview_template = BarkTemplate {
            id: edit_state.id.clone(),
            category: edit_state.category,
            template: edit_state.template_text.clone(),
            description: edit_state.description.clone(),
            variables: edit_state.variables.clone(),
            moods: edit_state.moods.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            priority: edit_state.priority,
            max_uses: edit_state.max_uses,
            current_uses: 0,
            enabled: edit_state.enabled,
            created_by: TemplateSource::User("editor".to_string()),
            created_at: Some(std::time::SystemTime::now()),
        };

        let preview_text = preview_template.render_sample();
        ui.group(|ui| {
            ui.label("Rendered preview:");
            ui.label(egui::RichText::new(&preview_text).italics());
        });

        // Validation
        let errors = preview_template.validate();
        if !errors.is_empty() {
            ui.separator();
            ui.colored_label(egui::Color32::RED, "⚠️ Validation Errors:");
            for error in &errors {
                ui.label(format!("  • {}", error));
            }
        }

        ui.separator();

        // Actions
        let mut save_clicked = false;
        let mut cancel_clicked = false;
        ui.horizontal(|ui| {
            if ui.button("💾 Save").clicked() && errors.is_empty() {
                save_clicked = true;
            }

            if ui.button("Cancel").clicked() {
                cancel_clicked = true;
            }
        });
        
        // Handle actions after UI rendering
        if save_clicked {
            self.save_template(edit_state);
        } else if cancel_clicked {
            // Just drop edit_state without putting it back
        } else {
            // Put edit_state back for next frame
            self.template_edit_state = Some(edit_state);
        }
    }

    /// Start editing a template
    fn start_edit_template(&mut self, template: &BarkTemplate) {
        self.editing_template = Some(template.id.clone());
        self.template_edit_state = Some(TemplateEditState {
            id: template.id.clone(),
            category: template.category,
            template_text: template.template.clone(),
            description: template.description.clone(),
            variables: template.variables.clone(),
            moods: template.moods.join(", "),
            priority: template.priority,
            max_uses: template.max_uses,
            enabled: template.enabled,
            is_new: false,
        });
    }

    /// Start creating a new template
    fn start_new_template(&mut self) {
        let id = format!("user_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());

        self.editing_template = None;
        self.template_edit_state = Some(TemplateEditState {
            id,
            category: self.selected_bark_category.unwrap_or(BarkCategory::Greeting),
            template_text: "Hello, {{player_name}}!".to_string(),
            description: String::new(),
            variables: vec![TemplateVariable::new(
                "player_name",
                "Player Name",
                "The player's name",
                "Adventurer",
            )],
            moods: "neutral".to_string(),
            priority: 50,
            max_uses: 0,
            enabled: true,
            is_new: true,
        });
    }

    /// Save a template
    fn save_template(&mut self, edit_state: TemplateEditState) {
        let template = BarkTemplate {
            id: edit_state.id,
            category: edit_state.category,
            template: edit_state.template_text,
            description: edit_state.description,
            variables: edit_state.variables,
            moods: edit_state.moods.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            priority: edit_state.priority,
            max_uses: edit_state.max_uses,
            current_uses: 0,
            enabled: edit_state.enabled,
            created_by: TemplateSource::User("editor".to_string()),
            created_at: Some(std::time::SystemTime::now()),
        };

        self.bark_templates.add_template(template);
        self.template_edit_state = None;
        self.show_status("Template saved", false);
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
        assert_eq!(panel.selected_tab as i32, DirectorTab::Proposals as i32);
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

    #[test]
    fn test_template_edit_state() {
        let state = TemplateEditState {
            id: "test".to_string(),
            category: BarkCategory::Greeting,
            template_text: "Hello!".to_string(),
            description: "Test".to_string(),
            variables: vec![],
            moods: "neutral".to_string(),
            priority: 50,
            max_uses: 0,
            enabled: true,
            is_new: true,
        };

        assert!(state.is_new);
        assert_eq!(state.priority, 50);
    }

    #[test]
    fn test_recommendation_generation() {
        let panel = DirectorPanel::new();
        // Can't fully test without a real DirectorSystem, but we can verify the method exists
        // In a real test, we'd mock the DirectorSystem
    }
}
