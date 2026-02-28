//! External input channel — mpsc-based event bridge for MIDI, OSC, and other external controllers.

use std::sync::mpsc;

/// Events from external controllers (MIDI, OSC, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum ExternalEvent {
    /// Set a macro to a specific value by name.
    MacroSet { name: String, value: f64 },
    /// Note-on for a track.
    NoteOn {
        track: String,
        note: u8,
        velocity: f32,
    },
    /// Note-off for a track.
    NoteOff { track: String, note: u8 },
    /// MIDI CC message.
    CC {
        channel: u8,
        controller: u8,
        value: u8,
    },
    /// Jump to a section by index.
    SectionJump(usize),
    /// Toggle a layer by index.
    LayerToggle(usize),
    /// Set BPM.
    BpmSet(f64),
    /// Toggle play/stop.
    PlayStop,
}

/// Sender half — clone this for MIDI/OSC threads.
pub type ExternalInputSender = mpsc::Sender<ExternalEvent>;

/// Receiver half — held by the TUI event loop.
pub struct ExternalInputReceiver {
    rx: mpsc::Receiver<ExternalEvent>,
}

impl ExternalInputReceiver {
    /// Non-blocking poll for the next external event.
    pub fn poll(&self) -> Option<ExternalEvent> {
        self.rx.try_recv().ok()
    }

    /// Drain all pending events.
    pub fn drain(&self) -> Vec<ExternalEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }
}

/// Create a new external input channel pair.
pub fn external_channel() -> (ExternalInputSender, ExternalInputReceiver) {
    let (tx, rx) = mpsc::channel();
    (tx, ExternalInputReceiver { rx })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_and_receive() {
        let (tx, rx) = external_channel();
        tx.send(ExternalEvent::MacroSet {
            name: "filter".to_string(),
            value: 0.5,
        })
        .unwrap();
        let event = rx.poll().unwrap();
        assert_eq!(
            event,
            ExternalEvent::MacroSet {
                name: "filter".to_string(),
                value: 0.5
            }
        );
    }

    #[test]
    fn poll_empty_returns_none() {
        let (_tx, rx) = external_channel();
        assert!(rx.poll().is_none());
    }

    #[test]
    fn multiple_events_in_order() {
        let (tx, rx) = external_channel();
        tx.send(ExternalEvent::SectionJump(0)).unwrap();
        tx.send(ExternalEvent::SectionJump(1)).unwrap();
        tx.send(ExternalEvent::LayerToggle(2)).unwrap();

        assert_eq!(rx.poll(), Some(ExternalEvent::SectionJump(0)));
        assert_eq!(rx.poll(), Some(ExternalEvent::SectionJump(1)));
        assert_eq!(rx.poll(), Some(ExternalEvent::LayerToggle(2)));
        assert_eq!(rx.poll(), None);
    }

    #[test]
    fn drain_collects_all() {
        let (tx, rx) = external_channel();
        tx.send(ExternalEvent::PlayStop).unwrap();
        tx.send(ExternalEvent::BpmSet(140.0)).unwrap();

        let events = rx.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], ExternalEvent::PlayStop);
        assert_eq!(events[1], ExternalEvent::BpmSet(140.0));
    }

    #[test]
    fn drain_empty_returns_empty() {
        let (_tx, rx) = external_channel();
        assert!(rx.drain().is_empty());
    }

    #[test]
    fn clone_sender() {
        let (tx, rx) = external_channel();
        let tx2 = tx.clone();
        tx.send(ExternalEvent::SectionJump(0)).unwrap();
        tx2.send(ExternalEvent::SectionJump(1)).unwrap();

        assert_eq!(rx.poll(), Some(ExternalEvent::SectionJump(0)));
        assert_eq!(rx.poll(), Some(ExternalEvent::SectionJump(1)));
    }

    #[test]
    fn cc_event() {
        let (tx, rx) = external_channel();
        tx.send(ExternalEvent::CC {
            channel: 1,
            controller: 74,
            value: 127,
        })
        .unwrap();
        let event = rx.poll().unwrap();
        assert_eq!(
            event,
            ExternalEvent::CC {
                channel: 1,
                controller: 74,
                value: 127
            }
        );
    }

    #[test]
    fn note_on_off() {
        let (tx, rx) = external_channel();
        tx.send(ExternalEvent::NoteOn {
            track: "bass".to_string(),
            note: 36,
            velocity: 0.8,
        })
        .unwrap();
        tx.send(ExternalEvent::NoteOff {
            track: "bass".to_string(),
            note: 36,
        })
        .unwrap();

        let events = rx.drain();
        assert_eq!(events.len(), 2);
    }
}
