pub struct Audio;

const SECONDS: usize = 120;
const MAX_SAMPLES: usize = 44100 * SECONDS;
pub type SampleType = f32;

static mut MUSIC_DATA: [SampleType; MAX_SAMPLES] = [0.0; MAX_SAMPLES];

impl intro_rs::Audio for Audio {
    fn new() -> Self where Self: Sized {
        unsafe {
            for (index, sample) in MUSIC_DATA.iter_mut().enumerate() {
                *sample = (index % 220) as f32 / 440.0f32 * 0.01f32;
            }
        }
        Self
    }
    
    fn data_mut(&self) -> &mut [f32] {
        unsafe {
            &mut MUSIC_DATA
        }
    }
}