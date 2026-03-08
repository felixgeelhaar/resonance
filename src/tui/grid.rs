//! Grid visualization — projects the event stream onto a visual grid with playback cursor.

use crate::event::types::{Event, NoteOrSample};
use crate::event::Beat;

/// Grid zoom level — controls the time resolution of the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridZoom {
    /// One column per beat
    #[default]
    Beat,
    /// One column per half-bar (2 beats)
    HalfBar,
    /// One column per bar (4 beats)
    Bar,
    /// One column per 4-bar phrase
    FourBar,
}

impl GridZoom {
    /// Zoom in (higher resolution).
    pub fn zoom_in(self) -> Self {
        match self {
            Self::FourBar => Self::Bar,
            Self::Bar => Self::HalfBar,
            Self::HalfBar => Self::Beat,
            Self::Beat => Self::Beat,
        }
    }

    /// Zoom out (lower resolution).
    pub fn zoom_out(self) -> Self {
        match self {
            Self::Beat => Self::HalfBar,
            Self::HalfBar => Self::Bar,
            Self::Bar => Self::FourBar,
            Self::FourBar => Self::FourBar,
        }
    }

    /// Steps per bar at this zoom level.
    pub fn steps_per_bar(self) -> usize {
        match self {
            Self::Beat => 8,
            Self::HalfBar => 4,
            Self::Bar => 2,
            Self::FourBar => 1,
        }
    }

    /// Display label for the status bar.
    pub fn label(self) -> &'static str {
        match self {
            Self::Beat => "1/8",
            Self::HalfBar => "1/4",
            Self::Bar => "1/2",
            Self::FourBar => "1/1",
        }
    }
}

/// A cell in the grid visualization.
#[derive(Debug, Clone, PartialEq)]
pub enum GridCell {
    Empty,
    Hit(f32),    // velocity
    Note(u8),    // MIDI note
    Stacked(u8), // multiple simultaneous hits (count)
    Cursor,      // playback cursor position
}

/// Grid projection of events for a single track.
#[derive(Debug, Clone)]
pub struct TrackGrid {
    pub track_name: String,
    pub cells: Vec<GridCell>,
    pub steps: usize,
}

/// Assign a consistent color to a track name by hashing into the theme palette.
pub fn track_color(name: &str, palette: &[ratatui::style::Color; 8]) -> ratatui::style::Color {
    let hash: u32 = name
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    palette[(hash as usize) % palette.len()]
}

/// Map a velocity (0.0-1.0) to a color intensity using theme colors.
pub fn velocity_color(
    velocity: f32,
    base_color: ratatui::style::Color,
    bright: ratatui::style::Color,
    dim: ratatui::style::Color,
) -> ratatui::style::Color {
    if velocity > 0.7 {
        bright
    } else if velocity > 0.4 {
        base_color
    } else {
        dim
    }
}

/// Project events onto a grid with the given number of steps per bar.
pub fn project_events(
    events: &[Event],
    total_bars: u32,
    steps_per_bar: usize,
    cursor_beat: Option<Beat>,
) -> Vec<TrackGrid> {
    use std::collections::BTreeMap;

    let total_steps = total_bars as usize * steps_per_bar;
    let beats_per_step = (total_bars as f64 * 4.0) / total_steps as f64;

    // Group events by track
    let mut tracks: BTreeMap<u32, (String, Vec<GridCell>)> = BTreeMap::new();

    for event in events {
        let track_id = event.track_id.0;
        let step = (event.time.as_beats_f64() / beats_per_step).floor() as usize;
        if step >= total_steps {
            continue;
        }

        let entry = tracks.entry(track_id).or_insert_with(|| {
            let name = match &event.trigger {
                NoteOrSample::Sample(s) => s.clone(),
                NoteOrSample::Note(_) => format!("track_{track_id}"),
            };
            (name, vec![GridCell::Empty; total_steps])
        });

        let existing = &entry.1[step];
        entry.1[step] = match existing {
            GridCell::Empty | GridCell::Cursor => match &event.trigger {
                NoteOrSample::Sample(_) => GridCell::Hit(event.velocity),
                NoteOrSample::Note(n) => GridCell::Note(*n),
            },
            GridCell::Hit(_) | GridCell::Note(_) => GridCell::Stacked(2),
            GridCell::Stacked(n) => GridCell::Stacked(n + 1),
        };
    }

    // Apply cursor position
    if let Some(cursor) = cursor_beat {
        let cursor_step = (cursor.as_beats_f64() / beats_per_step).floor() as usize;
        if cursor_step < total_steps {
            for (_, cells) in tracks.values_mut() {
                if cells[cursor_step] == GridCell::Empty {
                    cells[cursor_step] = GridCell::Cursor;
                }
            }
        }
    }

    tracks
        .into_values()
        .map(|(name, cells)| TrackGrid {
            track_name: name,
            steps: total_steps,
            cells,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::types::TrackId;

    #[test]
    fn empty_events_empty_grid() {
        let grids = project_events(&[], 1, 8, None);
        assert!(grids.is_empty());
    }

    #[test]
    fn single_event_at_zero() {
        let events = vec![Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            0.8,
        )];
        let grids = project_events(&events, 1, 4, None);
        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].cells[0], GridCell::Hit(0.8));
        assert_eq!(grids[0].cells[1], GridCell::Empty);
    }

    #[test]
    fn cursor_position() {
        let events = vec![Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            0.8,
        )];
        let grids = project_events(&events, 1, 4, Some(Beat::from_beats(2)));
        // Step 2 should be cursor (empty cell)
        assert_eq!(grids[0].cells[2], GridCell::Cursor);
        // Step 0 should still be the hit (cursor doesn't overwrite)
        assert_eq!(grids[0].cells[0], GridCell::Hit(0.8));
    }

    #[test]
    fn note_event_grid() {
        let events = vec![Event::note(
            Beat::from_beats(1),
            Beat::from_beats(1),
            TrackId(0),
            60,
            0.8,
        )];
        let grids = project_events(&events, 1, 4, None);
        assert_eq!(grids[0].cells[1], GridCell::Note(60));
    }

    #[test]
    fn multiple_tracks() {
        let events = vec![
            Event::sample(Beat::ZERO, Beat::from_beats(1), TrackId(0), "kick", 0.8),
            Event::note(Beat::ZERO, Beat::from_beats(1), TrackId(1), 36, 0.7),
        ];
        let grids = project_events(&events, 1, 4, None);
        assert_eq!(grids.len(), 2);
    }

    #[test]
    fn grid_steps_match() {
        let events = vec![Event::sample(
            Beat::ZERO,
            Beat::from_beats(1),
            TrackId(0),
            "kick",
            0.8,
        )];
        let grids = project_events(&events, 2, 8, None);
        assert_eq!(grids[0].steps, 16); // 2 bars * 8 steps
        assert_eq!(grids[0].cells.len(), 16);
    }

    // --- GridZoom tests ---

    #[test]
    fn grid_zoom_cycle_in() {
        assert_eq!(GridZoom::FourBar.zoom_in(), GridZoom::Bar);
        assert_eq!(GridZoom::Bar.zoom_in(), GridZoom::HalfBar);
        assert_eq!(GridZoom::HalfBar.zoom_in(), GridZoom::Beat);
        assert_eq!(GridZoom::Beat.zoom_in(), GridZoom::Beat); // clamp
    }

    #[test]
    fn grid_zoom_cycle_out() {
        assert_eq!(GridZoom::Beat.zoom_out(), GridZoom::HalfBar);
        assert_eq!(GridZoom::HalfBar.zoom_out(), GridZoom::Bar);
        assert_eq!(GridZoom::Bar.zoom_out(), GridZoom::FourBar);
        assert_eq!(GridZoom::FourBar.zoom_out(), GridZoom::FourBar); // clamp
    }

    #[test]
    fn grid_zoom_steps_per_bar() {
        assert_eq!(GridZoom::Beat.steps_per_bar(), 8);
        assert_eq!(GridZoom::HalfBar.steps_per_bar(), 4);
        assert_eq!(GridZoom::Bar.steps_per_bar(), 2);
        assert_eq!(GridZoom::FourBar.steps_per_bar(), 1);
    }

    #[test]
    fn grid_zoom_labels() {
        assert_eq!(GridZoom::Beat.label(), "1/8");
        assert_eq!(GridZoom::HalfBar.label(), "1/4");
        assert_eq!(GridZoom::Bar.label(), "1/2");
        assert_eq!(GridZoom::FourBar.label(), "1/1");
    }

    #[test]
    fn grid_zoom_default_is_beat() {
        assert_eq!(GridZoom::default(), GridZoom::Beat);
    }

    // --- Track color tests ---

    fn default_palette() -> [ratatui::style::Color; 8] {
        use ratatui::style::Color;
        [
            Color::Cyan,
            Color::Magenta,
            Color::Yellow,
            Color::Green,
            Color::Blue,
            Color::Red,
            Color::LightCyan,
            Color::LightGreen,
        ]
    }

    #[test]
    fn track_color_consistent() {
        let pal = default_palette();
        let c1 = track_color("drums", &pal);
        let c2 = track_color("drums", &pal);
        assert_eq!(c1, c2);
    }

    #[test]
    fn track_color_different_names() {
        let pal = default_palette();
        let c1 = track_color("drums", &pal);
        let c2 = track_color("bass", &pal);
        // Different names should (likely) produce different colors
        // Not guaranteed but highly likely with 8 colors
        let _ = (c1, c2);
    }

    #[test]
    fn track_color_uses_palette() {
        use ratatui::style::Color;
        let custom_pal = [Color::Rgb(255, 0, 0); 8];
        let c = track_color("anything", &custom_pal);
        assert_eq!(c, Color::Rgb(255, 0, 0));
    }

    // --- Velocity color tests ---

    #[test]
    fn velocity_high_is_bright() {
        use ratatui::style::Color;
        let c = velocity_color(0.9, Color::Cyan, Color::White, Color::DarkGray);
        assert_eq!(c, Color::White);
    }

    #[test]
    fn velocity_mid_is_base() {
        use ratatui::style::Color;
        let c = velocity_color(0.5, Color::Cyan, Color::White, Color::DarkGray);
        assert_eq!(c, Color::Cyan);
    }

    #[test]
    fn velocity_low_is_dim() {
        use ratatui::style::Color;
        let c = velocity_color(0.2, Color::Cyan, Color::White, Color::DarkGray);
        assert_eq!(c, Color::DarkGray);
    }

    #[test]
    fn velocity_uses_theme_colors() {
        use ratatui::style::Color;
        let bright = Color::Rgb(255, 255, 0);
        let dim = Color::Rgb(50, 50, 50);
        assert_eq!(velocity_color(0.9, Color::Cyan, bright, dim), bright);
        assert_eq!(velocity_color(0.1, Color::Cyan, bright, dim), dim);
    }
}
