//! Settings panel — in-app configuration for AI, MIDI, OSC, theme, and general settings.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::theme::Theme;

/// A tab in the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Theme,
    AI,
    MIDI,
    OSC,
}

impl SettingsTab {
    pub fn all() -> &'static [SettingsTab] {
        &[
            SettingsTab::General,
            SettingsTab::Theme,
            SettingsTab::AI,
            SettingsTab::MIDI,
            SettingsTab::OSC,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Theme => "Theme",
            SettingsTab::AI => "AI",
            SettingsTab::MIDI => "MIDI",
            SettingsTab::OSC => "OSC",
        }
    }

    pub fn next(&self) -> Self {
        let tabs = Self::all();
        let idx = tabs.iter().position(|t| t == self).unwrap_or(0);
        tabs[(idx + 1) % tabs.len()]
    }

    pub fn prev(&self) -> Self {
        let tabs = Self::all();
        let idx = tabs.iter().position(|t| t == self).unwrap_or(0);
        tabs[(idx + tabs.len() - 1) % tabs.len()]
    }

    pub fn index(&self) -> usize {
        Self::all().iter().position(|t| t == self).unwrap_or(0)
    }
}

/// The kind of value a settings field holds.
#[derive(Debug, Clone)]
pub enum FieldKind {
    Text(String),
    Toggle(bool),
    Select(Vec<String>, usize),
}

/// A single configurable field in the settings panel.
#[derive(Debug, Clone)]
pub struct SettingsField {
    pub label: String,
    pub key: String,
    pub kind: FieldKind,
    pub description: String,
}

/// The settings panel state.
#[derive(Debug, Clone)]
pub struct SettingsPanel {
    pub visible: bool,
    pub active_tab: SettingsTab,
    pub selected_field: usize,
    pub editing: bool,
    fields: Vec<Vec<SettingsField>>,
    pub dirty: bool,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            active_tab: SettingsTab::General,
            selected_field: 0,
            editing: false,
            fields: Self::build_default_fields(),
            dirty: false,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.load_from_configs();
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.load_from_configs();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.editing = false;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.selected_field = 0;
        self.editing = false;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
        self.selected_field = 0;
        self.editing = false;
    }

    pub fn next_field(&mut self) {
        let tab_idx = self.active_tab.index();
        let count = self.fields.get(tab_idx).map(|f| f.len()).unwrap_or(0);
        if count > 0 {
            self.selected_field = (self.selected_field + 1) % count;
        }
    }

    pub fn prev_field(&mut self) {
        let tab_idx = self.active_tab.index();
        let count = self.fields.get(tab_idx).map(|f| f.len()).unwrap_or(0);
        if count > 0 {
            self.selected_field = (self.selected_field + count - 1) % count;
        }
    }

    /// Toggle a boolean field or cycle a select field.
    pub fn toggle_field(&mut self) {
        let tab_idx = self.active_tab.index();
        if let Some(fields) = self.fields.get_mut(tab_idx) {
            if let Some(field) = fields.get_mut(self.selected_field) {
                match &mut field.kind {
                    FieldKind::Toggle(ref mut val) => {
                        *val = !*val;
                        self.dirty = true;
                    }
                    FieldKind::Select(options, ref mut idx) => {
                        if !options.is_empty() {
                            *idx = (*idx + 1) % options.len();
                            self.dirty = true;
                        }
                    }
                    FieldKind::Text(_) => {
                        self.editing = true;
                    }
                }
            }
        }
    }

    /// Start editing the current text field.
    pub fn start_editing(&mut self) {
        let tab_idx = self.active_tab.index();
        if let Some(fields) = self.fields.get(tab_idx) {
            if let Some(field) = fields.get(self.selected_field) {
                if matches!(field.kind, FieldKind::Text(_)) {
                    self.editing = true;
                }
            }
        }
    }

    pub fn insert_char(&mut self, c: char) {
        if !self.editing {
            return;
        }
        let tab_idx = self.active_tab.index();
        if let Some(fields) = self.fields.get_mut(tab_idx) {
            if let Some(field) = fields.get_mut(self.selected_field) {
                if let FieldKind::Text(ref mut text) = field.kind {
                    text.push(c);
                    self.dirty = true;
                }
            }
        }
    }

    pub fn backspace(&mut self) {
        if !self.editing {
            return;
        }
        let tab_idx = self.active_tab.index();
        if let Some(fields) = self.fields.get_mut(tab_idx) {
            if let Some(field) = fields.get_mut(self.selected_field) {
                if let FieldKind::Text(ref mut text) = field.kind {
                    text.pop();
                    self.dirty = true;
                }
            }
        }
    }

    pub fn stop_editing(&mut self) {
        self.editing = false;
    }

    /// Get the currently selected theme name (if on Theme tab).
    pub fn selected_theme_name(&self) -> Option<&str> {
        let tab_idx = SettingsTab::Theme.index();
        if let Some(fields) = self.fields.get(tab_idx) {
            if let Some(field) = fields.first() {
                if let FieldKind::Select(options, idx) = &field.kind {
                    return options.get(*idx).map(|s| s.as_str());
                }
            }
        }
        None
    }

    /// Load current config values from disk into fields.
    pub fn load_from_configs(&mut self) {
        self.fields = Self::build_default_fields();

        // Load AI config
        if let Some(config) = crate::ai::config::load_config() {
            let tab_idx = SettingsTab::AI.index();
            if let Some(fields) = self.fields.get_mut(tab_idx) {
                for field in fields.iter_mut() {
                    match field.key.as_str() {
                        "ai_enabled" => field.kind = FieldKind::Toggle(config.enabled),
                        "ai_provider" => field.kind = FieldKind::Text(config.provider.clone()),
                        "ai_api_url" => field.kind = FieldKind::Text(config.api_url.clone()),
                        "ai_api_key" => field.kind = FieldKind::Text(config.api_key.clone()),
                        "ai_model" => field.kind = FieldKind::Text(config.model.clone()),
                        _ => {}
                    }
                }
            }
        }

        // Load MIDI config
        if let Some(config) = crate::midi::MidiConfig::load() {
            let tab_idx = SettingsTab::MIDI.index();
            if let Some(fields) = self.fields.get_mut(tab_idx) {
                for field in fields.iter_mut() {
                    if field.key == "midi_device" {
                        field.kind =
                            FieldKind::Text(config.device_name.clone().unwrap_or_default());
                    }
                }
            }
        }

        self.dirty = false;
    }

    /// Save current field values back to YAML config files.
    pub fn save_to_configs(&self) -> Result<(), String> {
        let home = dirs::home_dir().ok_or("no home directory")?;
        let dir = home.join(".resonance");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        // Save AI config
        let tab_idx = SettingsTab::AI.index();
        if let Some(fields) = self.fields.get(tab_idx) {
            let mut enabled = false;
            let mut provider = String::new();
            let mut api_url = String::new();
            let mut api_key = String::new();
            let mut model = String::new();

            for field in fields {
                match field.key.as_str() {
                    "ai_enabled" => {
                        if let FieldKind::Toggle(v) = &field.kind {
                            enabled = *v;
                        }
                    }
                    "ai_provider" => {
                        if let FieldKind::Text(v) = &field.kind {
                            provider = v.clone();
                        }
                    }
                    "ai_api_url" => {
                        if let FieldKind::Text(v) = &field.kind {
                            api_url = v.clone();
                        }
                    }
                    "ai_api_key" => {
                        if let FieldKind::Text(v) = &field.kind {
                            api_key = v.clone();
                        }
                    }
                    "ai_model" => {
                        if let FieldKind::Text(v) = &field.kind {
                            model = v.clone();
                        }
                    }
                    _ => {}
                }
            }

            let yaml = format!(
                "enabled: {enabled}\nprovider: {provider}\napi_url: {api_url}\napi_key: {api_key}\nmodel: {model}\n"
            );
            std::fs::write(dir.join("ai.yaml"), yaml).map_err(|e| e.to_string())?;
        }

        // Save OSC config
        let tab_idx = SettingsTab::OSC.index();
        if let Some(fields) = self.fields.get(tab_idx) {
            for field in fields {
                if field.key == "osc_port" {
                    if let FieldKind::Text(v) = &field.kind {
                        if !v.is_empty() {
                            let yaml = format!("port: {v}\nmappings: []\n");
                            std::fs::write(dir.join("osc.yaml"), yaml)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the fields for the current tab.
    pub fn current_fields(&self) -> &[SettingsField] {
        let tab_idx = self.active_tab.index();
        self.fields
            .get(tab_idx)
            .map(|f| f.as_slice())
            .unwrap_or(&[])
    }

    /// Build the default set of fields for all tabs.
    fn build_default_fields() -> Vec<Vec<SettingsField>> {
        let theme_names: Vec<String> = super::theme::builtin::all_builtins()
            .iter()
            .map(|t| t.name.clone())
            .collect();

        vec![
            // General
            vec![
                SettingsField {
                    label: "Default BPM".into(),
                    key: "default_bpm".into(),
                    kind: FieldKind::Text("120".into()),
                    description: "Default tempo for new projects".into(),
                },
                SettingsField {
                    label: "Default Zoom".into(),
                    key: "default_zoom".into(),
                    kind: FieldKind::Select(vec!["1x".into(), "2x".into(), "4x".into()], 0),
                    description: "Default grid zoom level".into(),
                },
            ],
            // Theme
            vec![SettingsField {
                label: "Theme".into(),
                key: "theme_name".into(),
                kind: FieldKind::Select(theme_names, 0),
                description: "Color theme (live preview on change)".into(),
            }],
            // AI
            vec![
                SettingsField {
                    label: "Enabled".into(),
                    key: "ai_enabled".into(),
                    kind: FieldKind::Toggle(false),
                    description: "Enable AI-powered natural language commands".into(),
                },
                SettingsField {
                    label: "Provider".into(),
                    key: "ai_provider".into(),
                    kind: FieldKind::Text(String::new()),
                    description: "LLM provider (openai, anthropic, etc.)".into(),
                },
                SettingsField {
                    label: "API URL".into(),
                    key: "ai_api_url".into(),
                    kind: FieldKind::Text(String::new()),
                    description: "API base URL".into(),
                },
                SettingsField {
                    label: "API Key".into(),
                    key: "ai_api_key".into(),
                    kind: FieldKind::Text(String::new()),
                    description: "API key (stored in ~/.resonance/ai.yaml)".into(),
                },
                SettingsField {
                    label: "Model".into(),
                    key: "ai_model".into(),
                    kind: FieldKind::Text(String::new()),
                    description: "Model identifier (e.g., gpt-4)".into(),
                },
            ],
            // MIDI
            vec![
                SettingsField {
                    label: "Device Name".into(),
                    key: "midi_device".into(),
                    kind: FieldKind::Text(String::new()),
                    description: "MIDI input device name (leave empty for default)".into(),
                },
                SettingsField {
                    label: "Channel Filter".into(),
                    key: "midi_channel".into(),
                    kind: FieldKind::Text(String::new()),
                    description: "MIDI channel filter (1-16, empty for all)".into(),
                },
            ],
            // OSC
            vec![SettingsField {
                label: "Listen Port".into(),
                key: "osc_port".into(),
                kind: FieldKind::Text("9000".into()),
                description: "UDP port for incoming OSC messages".into(),
            }],
        ]
    }

    /// Render the settings panel as a centered overlay.
    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let width = (area.width * 60 / 100).max(50);
        let height = (area.height * 70 / 100).max(15);
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let overlay = Rect::new(x, y, width, height);

        let block =
            super::themed_overlay_block(" Settings \u{2014} Ctrl+S:save  Esc:close ", theme);
        let inner = block.inner(overlay);
        frame.render_widget(block, overlay);

        if inner.height < 4 || inner.width < 20 {
            return;
        }

        // Tab bar
        let tab_line: Vec<Span> = SettingsTab::all()
            .iter()
            .enumerate()
            .flat_map(|(i, tab)| {
                let selected = *tab == self.active_tab;
                let mut spans = Vec::new();
                if i > 0 {
                    spans.push(Span::styled(
                        " \u{2502} ",
                        Style::default().fg(theme.border),
                    ));
                }
                let style = if selected {
                    Style::default()
                        .fg(theme.border_focused)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.editor_line_number)
                };
                spans.push(Span::styled(tab.label(), style));
                spans
            })
            .collect();

        let tab_bar = Paragraph::new(Line::from(tab_line));
        let tab_area = Rect::new(inner.x, inner.y, inner.width, 1);
        frame.render_widget(tab_bar, tab_area);

        // Separator
        let sep_area = Rect::new(inner.x, inner.y + 1, inner.width, 1);
        let sep = Paragraph::new(Line::from(Span::styled(
            "\u{2500}".repeat(inner.width as usize),
            Style::default().fg(theme.border),
        )));
        frame.render_widget(sep, sep_area);

        // Fields
        let fields = self.current_fields();
        let field_start_y = inner.y + 2;
        let max_fields = (inner.height.saturating_sub(3)) as usize;

        for (i, field) in fields.iter().enumerate().take(max_fields) {
            let y = field_start_y + i as u16;
            let selected = i == self.selected_field;

            let selector = if selected { "\u{25B6} " } else { "  " };
            let selector_style = Style::default().fg(if selected {
                theme.border_focused
            } else {
                theme.editor_fg
            });

            let value_display = match &field.kind {
                FieldKind::Text(text) => {
                    if field.key == "ai_api_key" && !text.is_empty() {
                        if self.editing && selected {
                            text.clone()
                        } else {
                            "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
                                .to_string()
                        }
                    } else if self.editing && selected {
                        format!("{text}\u{2588}") // cursor
                    } else {
                        text.clone()
                    }
                }
                FieldKind::Toggle(val) => {
                    if *val {
                        "[\u{2713}] On".to_string()
                    } else {
                        "[ ] Off".to_string()
                    }
                }
                FieldKind::Select(options, idx) => {
                    let name = options.get(*idx).map(|s| s.as_str()).unwrap_or("?");
                    format!("\u{25C0} {name} \u{25B6}")
                }
            };

            let label_style = Style::default().fg(theme.macro_name);
            let value_style = if self.editing && selected {
                Style::default()
                    .fg(theme.editor_fg)
                    .add_modifier(Modifier::UNDERLINED)
            } else {
                Style::default().fg(theme.editor_fg)
            };

            let line = Line::from(vec![
                Span::styled(selector, selector_style),
                Span::styled(format!("{}: ", field.label), label_style),
                Span::styled(value_display, value_style),
            ]);

            let line_area = Rect::new(inner.x, y, inner.width, 1);
            frame.render_widget(Paragraph::new(line), line_area);
        }

        // Description at bottom
        if let Some(field) = fields.get(self.selected_field) {
            let desc_y = inner.y + inner.height - 1;
            let desc_area = Rect::new(inner.x, desc_y, inner.width, 1);
            let desc = Paragraph::new(Line::from(Span::styled(
                &field.description,
                Style::default().fg(theme.editor_line_number),
            )));
            frame.render_widget(desc, desc_area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_panel_default() {
        let panel = SettingsPanel::new();
        assert!(!panel.visible);
        assert_eq!(panel.active_tab, SettingsTab::General);
        assert_eq!(panel.selected_field, 0);
        assert!(!panel.editing);
    }

    #[test]
    fn toggle_visibility() {
        let mut panel = SettingsPanel::new();
        panel.toggle();
        assert!(panel.visible);
        panel.toggle();
        assert!(!panel.visible);
    }

    #[test]
    fn tab_navigation() {
        let mut panel = SettingsPanel::new();
        assert_eq!(panel.active_tab, SettingsTab::General);
        panel.next_tab();
        assert_eq!(panel.active_tab, SettingsTab::Theme);
        panel.next_tab();
        assert_eq!(panel.active_tab, SettingsTab::AI);
        panel.prev_tab();
        assert_eq!(panel.active_tab, SettingsTab::Theme);
    }

    #[test]
    fn tab_wraps_around() {
        let mut panel = SettingsPanel::new();
        panel.active_tab = SettingsTab::OSC;
        panel.next_tab();
        assert_eq!(panel.active_tab, SettingsTab::General);
    }

    #[test]
    fn field_navigation() {
        let mut panel = SettingsPanel::new();
        // General tab has 2 fields
        assert_eq!(panel.selected_field, 0);
        panel.next_field();
        assert_eq!(panel.selected_field, 1);
        panel.next_field();
        assert_eq!(panel.selected_field, 0); // wraps
        panel.prev_field();
        assert_eq!(panel.selected_field, 1);
    }

    #[test]
    fn toggle_field_boolean() {
        let mut panel = SettingsPanel::new();
        panel.active_tab = SettingsTab::AI;
        panel.selected_field = 0; // Enabled toggle
        panel.toggle_field();
        let fields = panel.current_fields();
        if let FieldKind::Toggle(val) = &fields[0].kind {
            assert!(*val);
        } else {
            panic!("expected Toggle");
        }
    }

    #[test]
    fn text_editing() {
        let mut panel = SettingsPanel::new();
        panel.active_tab = SettingsTab::AI;
        panel.selected_field = 1; // Provider text field
        panel.start_editing();
        assert!(panel.editing);
        panel.insert_char('o');
        panel.insert_char('a');
        panel.insert_char('i');
        let fields = panel.current_fields();
        if let FieldKind::Text(val) = &fields[1].kind {
            assert_eq!(val, "oai");
        }
        panel.backspace();
        let fields = panel.current_fields();
        if let FieldKind::Text(val) = &fields[1].kind {
            assert_eq!(val, "oa");
        }
        panel.stop_editing();
        assert!(!panel.editing);
    }

    #[test]
    fn select_field_cycles() {
        let mut panel = SettingsPanel::new();
        panel.active_tab = SettingsTab::Theme;
        panel.selected_field = 0; // Theme select
        let initial = if let FieldKind::Select(_, idx) = &panel.current_fields()[0].kind {
            *idx
        } else {
            panic!("expected Select");
        };
        panel.toggle_field();
        if let FieldKind::Select(_, idx) = &panel.current_fields()[0].kind {
            assert_eq!(*idx, initial + 1);
        }
    }

    #[test]
    fn all_tabs_have_fields() {
        let panel = SettingsPanel::new();
        for tab in SettingsTab::all() {
            panel.active_tab.index(); // ensure it works
            let idx = tab.index();
            assert!(!panel.fields[idx].is_empty(), "tab {:?} has no fields", tab);
        }
    }

    #[test]
    fn dirty_flag_set_on_edit() {
        let mut panel = SettingsPanel::new();
        assert!(!panel.dirty);
        panel.active_tab = SettingsTab::AI;
        panel.selected_field = 0;
        panel.toggle_field(); // toggle enabled
        assert!(panel.dirty);
    }

    #[test]
    fn selected_theme_name_returns_first_by_default() {
        let panel = SettingsPanel::new();
        let name = panel.selected_theme_name().unwrap();
        assert_eq!(name, "Default");
    }

    #[test]
    fn tab_labels() {
        assert_eq!(SettingsTab::General.label(), "General");
        assert_eq!(SettingsTab::AI.label(), "AI");
        assert_eq!(SettingsTab::OSC.label(), "OSC");
    }
}
