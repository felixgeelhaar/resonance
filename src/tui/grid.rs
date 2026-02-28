//! Grid visualization — projects the event stream onto a visual grid with playback cursor.

use crate::event::types::{Event, NoteOrSample};
use crate::event::Beat;

/// A cell in the grid visualization.
#[derive(Debug, Clone, PartialEq)]
pub enum GridCell {
    Empty,
    Hit(f32), // velocity
    Note(u8), // MIDI note
    Cursor,   // playback cursor position
}

/// Grid projection of events for a single track.
#[derive(Debug, Clone)]
pub struct TrackGrid {
    pub track_name: String,
    pub cells: Vec<GridCell>,
    pub steps: usize,
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

        entry.1[step] = match &event.trigger {
            NoteOrSample::Sample(_) => GridCell::Hit(event.velocity),
            NoteOrSample::Note(n) => GridCell::Note(*n),
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
}
