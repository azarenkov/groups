# DTLN ONNX models

The High noise-suppression pipeline loads two ONNX models from this
directory:

- `model_1.onnx` — spectral mask network (magnitude → mask).
- `model_2.onnx` — time-domain refinement network with LSTM state.

Without both files the High option still routes audio through the
worklet pipeline but skips inference (passthrough — audible).

Source: <https://github.com/breizhn/DTLN>

Expected I/O shapes per inference call (`frontend/src/services/dfn.ts`):

| Model     | Input                                | Output                                |
| --------- | ------------------------------------ | ------------------------------------- |
| `model_1` | mag `[1, 1, 257]`, state `[1,2,128,2]` | mask `[1, 1, 257]`, state `[1,2,128,2]` |
| `model_2` | time `[1, 1, 512]`, state `[1,2,128,2]` | out `[1, 1, 512]`, state `[1,2,128,2]`  |

Sample rate: 16 kHz, hop 128, window 512 (Hann), single combined LSTM
state tensor (rank 4, 512 floats) per model.

## Where to get the files

Pre-converted ONNX exports from upstream `breizhn/DTLN`:
<https://github.com/breizhn/DTLN/tree/master/pretrained_model/quantized>

Or convert the Keras `.h5` checkpoints with `tf2onnx`.
