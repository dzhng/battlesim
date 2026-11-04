//! A clip: frames at a fixed rate over a window of the reel, for video, with
//! the sound cut to the same window.

use native_client::audio::SAMPLE_RATE;

pub(super) struct Clip {
    pub fps: u32,
    from_ms: f64,
    to_ms: Option<f64>,
}

impl Clip {
    /// The clip `--fps`, `--from-ms` and `--to-ms` ask for; none without `--fps`.
    pub fn new(
        fps: Option<u32>,
        from_ms: Option<f64>,
        to_ms: Option<f64>,
    ) -> Result<Option<Self>, String> {
        let Some(fps) = fps else {
            if from_ms.is_some() || to_ms.is_some() {
                return Err("--from-ms and --to-ms need --fps".into());
            }
            return Ok(None);
        };
        let from_ms = from_ms.unwrap_or(0.0);
        if fps == 0 || from_ms < 0.0 || to_ms.is_some_and(|to| to <= from_ms) {
            return Err("a clip needs --fps above 0 and 0 <= --from-ms < --to-ms".into());
        }
        Ok(Some(Self {
            fps,
            from_ms,
            to_ms,
        }))
    }

    /// The reel time of frame `index`, or none once the clip has ended: at
    /// `--to-ms`, or else at the reel's last record, `last_ms`.
    pub fn frame_ms(&self, index: usize, last_ms: f64) -> Option<f64> {
        let at = self.from_ms + index as f64 * 1000.0 / f64::from(self.fps);
        (at < self.to_ms.unwrap_or(last_ms)).then_some(at)
    }

    /// The clip's window of the reel's sound: interleaved stereo from reel
    /// time 0.
    pub fn sound<'a>(&self, samples: &'a [f32]) -> &'a [f32] {
        let sample = |ms: f64| {
            let frame = (ms / 1000.0 * f64::from(SAMPLE_RATE)) as usize;
            (frame * 2).min(samples.len())
        };
        &samples[sample(self.from_ms)..self.to_ms.map_or(samples.len(), sample)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_fall_on_the_clock_from_the_start_to_the_end_of_the_window() {
        let clip = Clip::new(Some(120), Some(1000.0), Some(1050.0))
            .unwrap()
            .unwrap();
        let frames: Vec<f64> = (0..).map_while(|i| clip.frame_ms(i, 99_000.0)).collect();
        let expected: Vec<f64> = (0..6).map(|i| 1000.0 + i as f64 * 1000.0 / 120.0).collect();
        assert_eq!(
            frames, expected,
            "six frames, 1/120 s apart, none at or past 1050 ms"
        );
        let open = Clip::new(Some(30), None, None).unwrap().unwrap();
        assert_eq!(open.frame_ms(2, 100.0), Some(2000.0 / 30.0));
        assert!(
            open.frame_ms(3, 100.0).is_none(),
            "the reel's end closes an open clip"
        );
    }

    #[test]
    fn the_sound_is_cut_to_the_window() {
        let second = 2 * SAMPLE_RATE as usize;
        let samples: Vec<f32> = (0..3 * second).map(|i| i as f32).collect();
        let clip = Clip::new(Some(60), Some(500.0), Some(1500.0))
            .unwrap()
            .unwrap();
        let cut = clip.sound(&samples);
        assert_eq!(cut.len(), second);
        assert_eq!(cut[0], (second / 2) as f32);
        let open = Clip::new(Some(60), Some(2500.0), None).unwrap().unwrap();
        assert_eq!(open.sound(&samples).len(), second / 2);
    }

    #[test]
    fn a_window_needs_a_rate_and_a_forward_span() {
        assert!(Clip::new(None, Some(10.0), None).is_err());
        assert!(Clip::new(Some(0), None, None).is_err());
        assert!(Clip::new(Some(60), Some(500.0), Some(500.0)).is_err());
        assert!(Clip::new(None, None, None).unwrap().is_none());
    }
}
