use crate::frame::RawFrame;
use rayon::prelude::*;

const MAX_COMPONENTS: usize = 5;
const INITIAL_VARIANCE: f32 = 15.0;
const MATCH_THRESHOLD: f32 = 9.0;
const BACKGROUND_RATIO: f32 = 0.9;
const COMPLEXITY_THRESHOLD: f32 = 0.05;

#[derive(Clone, Copy, Default)]
struct Gaussian {
    weight: f32,
    mean: [f32; 3],
    variance: f32,
}

pub struct BackgroundSubtractorMOG2 {
    history: usize,
    var_threshold: f32,
    frames: usize,
    components: Vec<[Gaussian; MAX_COMPONENTS]>,
    mode_counts: Vec<u8>,
    foreground: Vec<u8>,
    background: RawFrame,
}

impl BackgroundSubtractorMOG2 {
    pub fn new(history: usize, var_threshold: f32) -> Self {
        Self {
            history: if history == 0 { 500 } else { history },
            var_threshold,
            frames: 0,
            components: vec![],
            mode_counts: vec![],
            foreground: vec![],
            background: RawFrame::default(),
        }
    }

    pub fn apply(&mut self, frame: &RawFrame) -> &RawFrame {
        if self.background.width != frame.width || self.background.height != frame.height {
            let pixels = frame.width as usize * frame.height as usize;
            self.background = RawFrame::new(frame.width, frame.height);
            self.components = vec![[Gaussian::default(); MAX_COMPONENTS]; pixels];
            self.mode_counts = vec![0; pixels];
            self.foreground = vec![0; pixels];
            self.frames = 0;
        }
        self.frames = self.frames.saturating_add(1);
        let learning_rate = 1.0 / self.frames.saturating_mul(2).min(self.history) as f32;
        let var_threshold = self.var_threshold;

        // Each pixel has its own model; buffers are reused and work is split across CPU cores.
        self.components.par_iter_mut().with_min_len(1024)
            .zip(self.mode_counts.par_iter_mut())
            .zip(frame.data.par_chunks_exact(3))
            .zip(self.background.data.par_chunks_exact_mut(3))
            .zip(self.foreground.par_iter_mut())
            .for_each(|((((components, mode_count), pixel), background), foreground)| {
                process_pixel(components, mode_count, pixel, background, foreground, learning_rate, var_threshold);
            });
        &self.background
    }

    pub fn foreground_mask(&self) -> &[u8] {
        &self.foreground
    }
}

fn process_pixel(components: &mut [Gaussian; MAX_COMPONENTS], mode_count: &mut u8, pixel: &[u8], background: &mut [u8], foreground: &mut u8, learning_rate: f32, var_threshold: f32) {
    let color = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];
    let decay = 1.0 - learning_rate;
    let prune = learning_rate * COMPLEXITY_THRESHOLD;
    let mut matched = false;
    let mut is_background = false;
    let mut total_weight = 0.0;
    let mut active_count = 0;

    for index in 0..*mode_count as usize {
        let mut component = components[index];
        let mut updated = false;
        component.weight = decay * component.weight - prune;
        if !matched {
            let difference = [color[0] - component.mean[0], color[1] - component.mean[1], color[2] - component.mean[2]];
            let distance = difference[0] * difference[0] + difference[1] * difference[1] + difference[2] * difference[2];
            if total_weight < BACKGROUND_RATIO && distance < var_threshold * component.variance {
                is_background = true;
            }
            if distance < MATCH_THRESHOLD * component.variance {
                matched = true;
                updated = true;
                component.weight += learning_rate;
                let gain = learning_rate / component.weight;
                for channel in 0..3 {
                    component.mean[channel] += gain * difference[channel];
                }
                component.variance = (component.variance + gain * (distance - component.variance)).clamp(4.0, 75.0);
            }
        }
        if component.weight < prune {
            continue;
        }
        total_weight += component.weight;
        // Insert the updated component by weight, moving at most four existing components.
        insert_component(components, active_count, component, updated);
        active_count += 1;
    }

    if total_weight > f32::EPSILON {
        let inverse_weight = 1.0 / total_weight;
        for component in components[..active_count].iter_mut() {
            component.weight *= inverse_weight;
        }
    }
    if !matched {
        let index = active_count.min(MAX_COMPONENTS - 1);
        active_count = (active_count + 1).min(MAX_COMPONENTS);
        for component in components[..index].iter_mut() {
            component.weight *= decay;
        }
        let component = Gaussian {
            weight: if active_count == 1 { 1.0 } else { learning_rate },
            mean: color,
            variance: INITIAL_VARIANCE,
        };
        insert_component(components, index, component, true);
    }
    *mode_count = active_count as u8;
    *foreground = if is_background { 0 } else { 255 };

    // Recover the background as a weighted mean of the strongest components.
    let mut mean = [0.0; 3];
    let mut weight = 0.0;
    for component in components[..active_count].iter() {
        for channel in 0..3 {
            mean[channel] += component.weight * component.mean[channel];
        }
        weight += component.weight;
        if weight > BACKGROUND_RATIO {
            break;
        }
    }
    let inverse_weight = if weight > f32::EPSILON { 1.0 / weight } else { 0.0 };
    for channel in 0..3 {
        background[channel] = (mean[channel] * inverse_weight).round_ties_even().clamp(0.0, 255.0) as u8;
    }
}

fn insert_component(components: &mut [Gaussian; MAX_COMPONENTS], mut index: usize, component: Gaussian, move_equal: bool) {
    while index > 0 && (component.weight > components[index - 1].weight || (move_equal && component.weight == components[index - 1].weight)) {
        components[index] = components[index - 1];
        index -= 1;
    }
    components[index] = component;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_frame(width: u32, height: u32, color: [u8; 3]) -> RawFrame {
        RawFrame {
            data: color.repeat(width as usize * height as usize),
            width,
            height,
        }
    }

    #[test]
    fn first_frame_initializes_background() {
        let frame = solid_frame(2, 1, [10, 80, 200]);
        let mut mog2 = BackgroundSubtractorMOG2::new(30, 16.0);
        assert_eq!(mog2.apply(&frame).data, frame.data);
        assert_eq!(mog2.foreground_mask(), &[255, 255]);
        assert_eq!(mog2.apply(&frame).data, frame.data);
        assert_eq!(mog2.foreground_mask(), &[0, 0]);
    }

    #[test]
    fn transient_foreground_does_not_replace_background() {
        let frame = solid_frame(1, 1, [20, 40, 60]);
        let moving_object = solid_frame(1, 1, [220, 180, 140]);
        let mut mog2 = BackgroundSubtractorMOG2::new(30, 16.0);
        for _ in 0..30 {
            mog2.apply(&frame);
        }
        assert_eq!(mog2.apply(&moving_object).data, frame.data);
        assert_eq!(mog2.foreground_mask(), &[255]);
        assert_eq!(mog2.apply(&frame).data, frame.data);
        assert_eq!(mog2.foreground_mask(), &[0]);
    }

    #[test]
    fn stationary_foreground_becomes_background() {
        let frame = solid_frame(1, 1, [20, 40, 60]);
        let stopped_object = solid_frame(1, 1, [220, 180, 140]);
        let mut mog2 = BackgroundSubtractorMOG2::new(30, 16.0);
        for _ in 0..30 {
            mog2.apply(&frame);
        }
        for _ in 0..150 {
            mog2.apply(&stopped_object);
        }
        assert_eq!(mog2.apply(&stopped_object).data, stopped_object.data);
        assert_eq!(mog2.foreground_mask(), &[0]);
    }

    #[test]
    fn resolution_change_resets_pixel_models() {
        let mut mog2 = BackgroundSubtractorMOG2::new(30, 16.0);
        for _ in 0..30 {
            mog2.apply(&solid_frame(2, 1, [10, 20, 30]));
        }
        let frame = solid_frame(1, 2, [100, 150, 200]);
        let background = mog2.apply(&frame);
        assert_eq!((background.width, background.height), (1, 2));
        assert_eq!(background.data, frame.data);
        assert_eq!(mog2.foreground_mask(), &[255, 255]);
    }

    #[test]
    fn alternating_background_retains_multiple_components() {
        let first = solid_frame(1, 1, [10, 10, 10]);
        let second = solid_frame(1, 1, [200, 200, 200]);
        let mut mog2 = BackgroundSubtractorMOG2::new(30, 16.0);
        for _ in 0..100 {
            mog2.apply(&first);
            mog2.apply(&second);
        }
        mog2.apply(&first);
        assert_eq!(mog2.foreground_mask(), &[0]);
        mog2.apply(&second);
        assert_eq!(mog2.foreground_mask(), &[0]);
        assert_eq!(mog2.mode_counts, vec![2]);
    }
}
