//! Commands sent from the main thread to the audio thread via ring buffer.

/// Commands sent from the main thread to the audio thread via ring buffer.
#[derive(Debug)]
pub enum AudioCommand {
    /// Push rendered audio samples into the playback buffer.
    /// Contains interleaved stereo samples (L, R, L, R, ...).
    Samples(Vec<f32>),

    /// Set master volume (0.0 to 1.0).
    SetVolume(f32),

    /// Stop playback and clear buffers.
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ringbuf::{
        traits::{Consumer, Producer, Split},
        HeapRb,
    };

    #[test]
    fn test_command_send_receive_samples() {
        let rb = HeapRb::<AudioCommand>::new(16);
        let (mut prod, mut cons) = rb.split();

        let samples = vec![0.1, -0.2, 0.3, -0.4];
        prod.try_push(AudioCommand::Samples(samples.clone()))
            .unwrap();

        let cmd = cons.try_pop().unwrap();
        match cmd {
            AudioCommand::Samples(data) => assert_eq!(data, samples),
            _ => panic!("expected Samples command"),
        }
    }

    #[test]
    fn test_command_send_receive_volume() {
        let rb = HeapRb::<AudioCommand>::new(16);
        let (mut prod, mut cons) = rb.split();

        prod.try_push(AudioCommand::SetVolume(0.75)).unwrap();

        let cmd = cons.try_pop().unwrap();
        match cmd {
            AudioCommand::SetVolume(v) => assert!((v - 0.75).abs() < f32::EPSILON),
            _ => panic!("expected SetVolume command"),
        }
    }

    #[test]
    fn test_command_send_receive_stop() {
        let rb = HeapRb::<AudioCommand>::new(16);
        let (mut prod, mut cons) = rb.split();

        prod.try_push(AudioCommand::Stop).unwrap();

        let cmd = cons.try_pop().unwrap();
        assert!(matches!(cmd, AudioCommand::Stop));
    }

    #[test]
    fn test_command_ordering_preserved() {
        let rb = HeapRb::<AudioCommand>::new(16);
        let (mut prod, mut cons) = rb.split();

        prod.try_push(AudioCommand::SetVolume(0.5)).unwrap();
        prod.try_push(AudioCommand::Samples(vec![1.0, 2.0]))
            .unwrap();
        prod.try_push(AudioCommand::Stop).unwrap();

        assert!(matches!(
            cons.try_pop().unwrap(),
            AudioCommand::SetVolume(_)
        ));
        assert!(matches!(cons.try_pop().unwrap(), AudioCommand::Samples(_)));
        assert!(matches!(cons.try_pop().unwrap(), AudioCommand::Stop));
        assert!(cons.try_pop().is_none());
    }
}
