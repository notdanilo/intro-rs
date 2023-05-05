#![no_std]

// some useful song defines for 4klang
pub const SAMPLE_RATE: usize = 44100;
pub const BPM: f32 = 125.000000;
pub const MAX_INSTRUMENTS: usize = 6;
pub const MAX_PATTERNS: usize = 14;
pub const PATTERN_SIZE_SHIFT: usize = 4;
pub const PATTERN_SIZE: usize = 1 << PATTERN_SIZE_SHIFT;
pub const MAX_TICKS: usize = MAX_PATTERNS * PATTERN_SIZE;
pub const SAMPLES_PER_TICK: usize = 5292;
pub const MAX_SAMPLES: usize = SAMPLES_PER_TICK * MAX_TICKS;
pub const POLYPHONY: usize = 1;
pub type SampleType = f32;

extern "stdcall" {
    pub fn _4klang_render(data: *mut f32);
}