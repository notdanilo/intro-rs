pub struct Audio;

use four_klang::MAX_SAMPLES;
use four_klang::SampleType;

static mut MUSIC_DATA: [SampleType; MAX_SAMPLES] = [0.0; MAX_SAMPLES];

impl intro_rs::Audio for Audio {
    fn new() -> Self where Self: Sized {
        unsafe {
            four_klang::_4klang_render(MUSIC_DATA.as_mut_ptr());
        }
        Self
    }
    
    fn data_mut(&self) -> &mut [f32] {
        unsafe {
            &mut MUSIC_DATA
        }
    }
}