use df::tract::{DfParams, DfTract, RuntimeParams};
use ndarray::{ArrayView2, ArrayViewMut2};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Denoiser {
    model: DfTract,
}

#[wasm_bindgen]
impl Denoiser {
    #[wasm_bindgen(constructor)]
    pub fn new(model: &[u8], atten_lim_db: f32) -> Result<Denoiser, JsError> {
        let params = DfParams::from_bytes(model).map_err(|e| JsError::new(&e.to_string()))?;
        let runtime = RuntimeParams::default_with_ch(1).with_atten_lim(atten_lim_db);
        let model = DfTract::new(params, &runtime).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Denoiser { model })
    }

    #[wasm_bindgen(getter = frameLength)]
    pub fn frame_length(&self) -> usize {
        self.model.hop_size
    }

    #[wasm_bindgen(getter = latency)]
    pub fn latency(&self) -> usize {
        self.model.fft_size - self.model.hop_size + self.model.lookahead * self.model.hop_size
    }

    #[wasm_bindgen(getter = sampleRate)]
    pub fn sample_rate(&self) -> usize {
        self.model.sr
    }

    #[wasm_bindgen(js_name = setAttenuationLimit)]
    pub fn set_attenuation_limit(&mut self, db: f32) {
        self.model.set_atten_lim(db);
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> Result<f32, JsError> {
        let hop = self.model.hop_size;
        if input.len() != hop || output.len() != hop {
            return Err(JsError::new("frame length mismatch"));
        }
        let noisy = ArrayView2::from_shape((1, hop), input).map_err(|e| JsError::new(&e.to_string()))?;
        let enhanced =
            ArrayViewMut2::from_shape((1, hop), output).map_err(|e| JsError::new(&e.to_string()))?;
        self.model.process(noisy, enhanced).map_err(|e| JsError::new(&e.to_string()))
    }
}
