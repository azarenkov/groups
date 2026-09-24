# DeepFilterNet3

Speech enhancement model used for the "High" noise suppression mode. It runs in a Web Worker through the upstream `libDF` streaming runtime (tract) compiled to WebAssembly, on 48 kHz audio in 10 ms frames, with 30 ms of algorithmic latency.

- Model: `DeepFilterNet3_onnx.tar.gz` from <https://github.com/Rikorose/DeepFilterNet/tree/d375b2d8309e0935d165700c91da9de862a99c31/models>
- SHA-256: `c94d91f70911001c946e0fabb4aa9adc37045f45a03b56008cb0c8244cb63616`
- Runtime: `frontend/wasm/deepfilter`, built by `scripts/voice/build-deepfilter.sh` into `frontend/src/shared/lib/deepfilter`
- Authors: Hendrik Schröter et al., "DeepFilterNet: Perceptually Motivated Real-Time Speech Enhancement", Interspeech 2023
- License: MIT or Apache-2.0, at your option; see `LICENSE-MIT` and `LICENSE-APACHE` in this directory
