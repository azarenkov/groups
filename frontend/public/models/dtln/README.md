# DTLN ONNX model

Drop the exported DTLN model here as `dtln.onnx`.

Without this file the High noise suppression option still routes audio through the worklet pipeline but bypasses inference (passthrough).

Source: https://github.com/breizhn/DTLN

Expected model shape:
- Input: `Float32Array` of 512 samples at 16 kHz (32 ms frame).
- Output: same shape, denoised PCM.

If the model has separate state inputs/outputs (LSTM hidden states), the
single-call wrapper in `frontend/src/services/dfn.ts` needs to be extended
to thread state between frames.

## Where to get it

- Hugging Face / community ONNX exports of `breizhn/DTLN`.
- Or convert the official Keras `.h5` checkpoint to ONNX with `tf2onnx`:
  ```bash
  pip install tf2onnx tensorflow
  python -m tf2onnx.convert --keras DTLN_norm_500h.h5 --output dtln.onnx
  ```
