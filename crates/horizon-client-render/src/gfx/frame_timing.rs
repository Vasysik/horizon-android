//! Opt-in top-level GPU timings without splitting the world's render pass.
use super::timing::{GpuTimer, Target};
use std::collections::VecDeque;

pub(super) struct FrameTiming {
    timers: [GpuTimer; 3],
    history: [VecDeque<f32>; 3],
    ready: Vec<f32>,
}
impl FrameTiming {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        // Encoder timestamps need no extra marker render passes on mobile tilers.
        if !device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            return None;
        }
        Some(Self {
            timers: [
                GpuTimer::new(device, queue)?,
                GpuTimer::new(device, queue)?,
                GpuTimer::new(device, queue)?,
            ],
            history: Default::default(),
            ready: Vec::new(),
        })
    }
    pub(super) fn begin_prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.timers[0].begin(encoder);
    }
    pub(super) fn begin_world(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: Target<'_>,
    ) {
        self.timers[0].end(device, encoder, target);
        self.timers[1].begin(encoder);
    }
    pub(super) fn screen(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: Target<'_>,
    ) {
        self.timers[1].end(device, encoder, target);
        self.timers[2].begin(encoder);
    }
    pub(super) fn end(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: Target<'_>,
    ) {
        self.timers[2].end(device, encoder, target);
    }
    pub(super) fn submitted(&mut self, queue: &wgpu::Queue) {
        for (timer, history) in self.timers.iter_mut().zip(&mut self.history) {
            timer.submitted(queue);
            self.ready.clear();
            timer.collect_ready(&mut self.ready);
            for sample in self.ready.drain(..).filter(|s| s.is_finite() && *s >= 0.0) {
                if history.len() == 120 {
                    history.pop_front();
                }
                history.push_back(sample);
            }
        }
    }
    pub(super) fn distributions(&self) -> Vec<(&'static str, f32, f32, f32, usize)> {
        ["prepare_uploads", "world", "post_hand_hud"]
            .into_iter()
            .zip(&self.history)
            .filter_map(|(name, h)| {
                let mut samples: Vec<_> = h.iter().copied().collect();
                if samples.is_empty() {
                    return None;
                }
                samples.sort_by(f32::total_cmp);
                let at = |p: usize| samples[((samples.len() - 1) * p) / 100];
                Some((name, at(50), at(95), at(99), samples.len()))
            })
            .collect()
    }
    pub(super) fn summaries(&self) -> Vec<(&'static str, f32, f32, usize)> {
        ["prepare_uploads", "world", "post_hand_hud"]
            .into_iter()
            .zip(&self.history)
            .filter_map(|(name, history)| {
                if history.is_empty() {
                    return None;
                }
                let mut samples: Vec<_> = history.iter().copied().collect();
                samples.sort_by(f32::total_cmp);
                Some((
                    name,
                    samples[samples.len() / 2],
                    *samples.last()?,
                    samples.len(),
                ))
            })
            .collect()
    }
}
