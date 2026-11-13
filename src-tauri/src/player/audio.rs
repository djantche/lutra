//! Sound: the core's samples, resampled to the sound card's rate and played through `cpal`
//! (ALSA, so PipeWire or PulseAudio behind it).
//!
//! A core produces samples at its console's rate (32 768 Hz on a Game Boy Advance, 32 040.5 Hz
//! on a Super Nintendo) while the picture is paced by Lutra's clock: the two drift apart. The
//! resampling ratio is nudged by a fraction of a percent to keep the buffer half full (dynamic
//! rate control), which no ear hears, instead of letting it run dry or overflow.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

/// Buffered sound Lutra aims for: low enough to feel instant, high enough to never crackle.
const TARGET_LATENCY: f64 = 0.064;
/// How far the ratio may move to keep the buffer at its target.
const MAX_CORRECTION: f64 = 0.005;

type Buffer = Arc<Mutex<VecDeque<f32>>>;

pub struct Audio {
    _stream: cpal::Stream,
    buffer: Buffer,
    in_rate: f64,
    out_rate: f64,
    /// Where the next output sample falls in the input, counted from `previous`.
    position: f64,
    previous: [f32; 2],
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Buffer,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let channels = config.channels as usize;
    device.build_output_stream(
        config,
        move |out: &mut [T], _| {
            let mut queue = buffer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for frame in out.chunks_mut(channels) {
                // Running dry plays silence rather than old sound.
                let (left, right) = match (queue.pop_front(), queue.pop_front()) {
                    (Some(left), Some(right)) => (left, right),
                    _ => (0.0, 0.0),
                };
                for (index, sample) in frame.iter_mut().enumerate() {
                    *sample = T::from_sample(match index {
                        0 => left,
                        1 => right,
                        _ => 0.0,
                    });
                }
            }
        },
        |error| eprintln!("sound: {error}"),
        None,
    )
}

impl Audio {
    /// Opens the default sound output for a core producing `in_rate` samples a second.
    pub fn open(in_rate: f64) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no sound output")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let buffer: Buffer = Arc::default();
        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, buffer.clone()),
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, buffer.clone()),
            cpal::SampleFormat::I32 => build::<i32>(&device, &config, buffer.clone()),
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, buffer.clone()),
            other => return Err(format!("unsupported sound format {other}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            buffer,
            in_rate: if in_rate > 0.0 { in_rate } else { 44_100.0 },
            out_rate: f64::from(config.sample_rate.0),
            position: 0.0,
            previous: [0.0; 2],
        })
    }

    /// The core changed its timing.
    pub fn set_input_rate(&mut self, rate: f64) {
        if rate > 0.0 {
            self.in_rate = rate;
        }
    }

    /// Queues interleaved stereo samples from the core.
    pub fn push(&mut self, samples: &[i16]) {
        if samples.len() < 2 {
            return;
        }
        let target = self.out_rate * TARGET_LATENCY * 2.0;
        let filled = self.buffer.lock().map(|queue| queue.len()).unwrap_or(0) as f64;
        let correction =
            (MAX_CORRECTION * (target - filled) / target).clamp(-MAX_CORRECTION, MAX_CORRECTION);
        let step = self.in_rate / (self.out_rate * (1.0 + correction));

        let (output, position, previous) = resample(samples, step, self.position, self.previous);
        self.position = position;
        self.previous = previous;

        if let Ok(mut queue) = self.buffer.lock() {
            queue.extend(output);
            // Far behind (the game paused, the machine stalled): drop the oldest sound.
            let limit = (target * 3.0) as usize;
            if queue.len() > limit {
                let excess = queue.len() - limit;
                queue.drain(..excess);
            }
        }
    }
}

/// Linear resampling of interleaved stereo `samples`, `step` input frames per output frame.
/// `position` and `previous` carry over from the last call; their next values are returned.
fn resample(
    samples: &[i16],
    step: f64,
    mut position: f64,
    previous: [f32; 2],
) -> (Vec<f32>, f64, [f32; 2]) {
    let frames = samples.len() / 2;
    let frame = |index: usize| -> [f32; 2] {
        // Index 0 is the last frame of the previous call.
        if index == 0 {
            previous
        } else {
            let at = (index - 1) * 2;
            [
                f32::from(samples[at]) / 32768.0,
                f32::from(samples[at + 1]) / 32768.0,
            ]
        }
    };
    let mut output = Vec::with_capacity(((frames as f64 / step) as usize + 2) * 2);
    while position < frames as f64 {
        let index = position.floor() as usize;
        let fraction = (position - index as f64) as f32;
        let (a, b) = (frame(index), frame(index + 1));
        output.push(a[0] + (b[0] - a[0]) * fraction);
        output.push(a[1] + (b[1] - a[1]) * fraction);
        position += step;
    }
    (output, position - frames as f64, frame(frames))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampling_keeps_the_rate_and_continues_across_calls() {
        // 32 768 Hz to 48 000 Hz: 1 000 input frames give about 1 465 output frames.
        let samples = vec![1000i16; 2000];
        let step = 32_768.0 / 48_000.0;
        let (first, position, previous) = resample(&samples, step, 0.0, [0.0; 2]);
        let (second, _, _) = resample(&samples, step, position, previous);
        let total = (first.len() + second.len()) / 2;
        assert!((2925..=2935).contains(&total), "{total}");
        // After the first frame, a constant signal stays constant.
        assert!(
            second
                .iter()
                .all(|sample| (sample - 1000.0 / 32768.0).abs() < 1e-6)
        );
    }
}
