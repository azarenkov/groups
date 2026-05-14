import * as ort from "onnxruntime-web";
import FFT from "fft.js";

ort.env.wasm.numThreads = 1;
ort.env.wasm.wasmPaths = assetUrl("ort/");

const HOP = 128;
const WINDOW = 512;
const FREQ_BINS = WINDOW / 2 + 1;
const STATE_SHAPE: number[] = [1, 2, 128, 2];
const STATE_SIZE = STATE_SHAPE.reduce((a, b) => a * b, 1);

function assetBase(): string {
  const electron = (window as unknown as { electron?: { appBase?: string } })
    .electron;
  if (electron?.appBase) return electron.appBase;
  return window.location.href.replace(/[^/]*$/, "");
}

function assetUrl(rel: string): string {
  return new URL(rel, assetBase()).toString();
}

const MODEL_1_URL = assetUrl("models/dtln/model_1.onnx");
const MODEL_2_URL = assetUrl("models/dtln/model_2.onnx");

export const DENOISER_SAMPLE_RATE = 16000;

export interface DfnPipeline {
  destination: MediaStreamAudioDestinationNode;
  outputTrack: MediaStreamTrack;
  setStrength: (value: number) => void;
  dispose: () => Promise<void>;
}

interface BuildOptions {
  source: MediaStreamAudioSourceNode;
  context: AudioContext;
}

class DtlnSession {
  private model1: ort.InferenceSession;
  private model2: ort.InferenceSession;
  private fft = new FFT(WINDOW);
  private hann: Float32Array;
  private inputBuffer = new Float32Array(WINDOW);
  private outputBuffer = new Float32Array(WINDOW);
  private spectrum: number[];
  private state1: Float32Array;
  private state2: Float32Array;
  private windowed = new Float32Array(WINDOW);
  private mag = new Float32Array(FREQ_BINS);
  private phaseRe = new Float32Array(FREQ_BINS);
  private phaseIm = new Float32Array(FREQ_BINS);
  private masked: number[];
  private ifftOut: number[];
  private timeFrame = new Float32Array(WINDOW);
  private m1Inputs: readonly string[];
  private m1Outputs: readonly string[];
  private m2Inputs: readonly string[];
  private m2Outputs: readonly string[];

  constructor(model1: ort.InferenceSession, model2: ort.InferenceSession) {
    this.model1 = model1;
    this.model2 = model2;
    this.m1Inputs = model1.inputNames;
    this.m1Outputs = model1.outputNames;
    this.m2Inputs = model2.inputNames;
    this.m2Outputs = model2.outputNames;

    this.hann = new Float32Array(WINDOW);
    for (let i = 0; i < WINDOW; i++) {
      this.hann[i] = 0.5 * (1 - Math.cos((2 * Math.PI * i) / (WINDOW - 1)));
    }

    this.spectrum = this.fft.createComplexArray();
    this.masked = this.fft.createComplexArray();
    this.ifftOut = this.fft.createComplexArray();

    this.state1 = new Float32Array(STATE_SIZE);
    this.state2 = new Float32Array(STATE_SIZE);
  }

  async process(hopSamples: Float32Array): Promise<Float32Array> {
    this.inputBuffer.copyWithin(0, HOP);
    this.inputBuffer.set(hopSamples, WINDOW - HOP);

    for (let i = 0; i < WINDOW; i++) {
      this.windowed[i] = this.inputBuffer[i] * this.hann[i];
    }
    this.fft.realTransform(this.spectrum, this.windowed);
    this.fft.completeSpectrum(this.spectrum);

    for (let i = 0; i < FREQ_BINS; i++) {
      const re = this.spectrum[i * 2];
      const im = this.spectrum[i * 2 + 1];
      const m = Math.sqrt(re * re + im * im);
      this.mag[i] = m;
      const inv = m > 1e-10 ? 1 / m : 0;
      this.phaseRe[i] = re * inv;
      this.phaseIm[i] = im * inv;
    }

    const mask = await this.runModel1(this.mag);

    for (let i = 0; i < FREQ_BINS; i++) {
      const m = this.mag[i] * mask[i];
      this.masked[i * 2] = m * this.phaseRe[i];
      this.masked[i * 2 + 1] = m * this.phaseIm[i];
    }
    for (let i = 1; i < FREQ_BINS - 1; i++) {
      const mirror = WINDOW - i;
      this.masked[mirror * 2] = this.masked[i * 2];
      this.masked[mirror * 2 + 1] = -this.masked[i * 2 + 1];
    }

    this.fft.inverseTransform(this.ifftOut, this.masked);
    for (let i = 0; i < WINDOW; i++)
      this.timeFrame[i] = this.ifftOut[i * 2] / WINDOW;

    const refined = await this.runModel2(this.timeFrame);

    this.outputBuffer.copyWithin(0, HOP);
    for (let i = WINDOW - HOP; i < WINDOW; i++) this.outputBuffer[i] = 0;
    for (let i = 0; i < WINDOW; i++) this.outputBuffer[i] += refined[i];

    return new Float32Array(this.outputBuffer.subarray(0, HOP));
  }

  private async runModel1(mag: Float32Array): Promise<Float32Array> {
    const feeds: Record<string, ort.Tensor> = {
      [this.m1Inputs[0]]: new ort.Tensor("float32", mag, [1, 1, FREQ_BINS]),
      [this.m1Inputs[1]]: new ort.Tensor("float32", this.state1, STATE_SHAPE.slice()),
    };
    const result = await this.model1.run(feeds);
    const mask = new Float32Array(result[this.m1Outputs[0]].data as Float32Array);
    this.state1 = new Float32Array(
      result[this.m1Outputs[1]].data as Float32Array,
    );
    return mask;
  }

  private async runModel2(timeFrame: Float32Array): Promise<Float32Array> {
    const feeds: Record<string, ort.Tensor> = {
      [this.m2Inputs[0]]: new ort.Tensor("float32", timeFrame, [1, 1, WINDOW]),
      [this.m2Inputs[1]]: new ort.Tensor("float32", this.state2, STATE_SHAPE.slice()),
    };
    const result = await this.model2.run(feeds);
    const refined = new Float32Array(
      result[this.m2Outputs[0]].data as Float32Array,
    );
    this.state2 = new Float32Array(
      result[this.m2Outputs[1]].data as Float32Array,
    );
    return refined;
  }
}

interface LoadedModels {
  model1: ort.InferenceSession;
  model2: ort.InferenceSession;
}

let modelsPromise: Promise<LoadedModels | null> | null = null;

async function loadModels(): Promise<LoadedModels | null> {
  if (modelsPromise) return modelsPromise;
  const p: Promise<LoadedModels | null> = (async () => {
    try {
      console.log("[dtln] HEAD probe", MODEL_1_URL, MODEL_2_URL);
      const head1 = await fetch(MODEL_1_URL, { method: "HEAD" });
      const head2 = await fetch(MODEL_2_URL, { method: "HEAD" });
      console.log("[dtln] HEAD probe results", head1.status, head2.status);
      if (!head1.ok || !head2.ok) return null;
      console.log("[dtln] creating InferenceSession (model_1)");
      const model1 = await ort.InferenceSession.create(MODEL_1_URL, {
        executionProviders: ["wasm"],
      });
      console.log("[dtln] creating InferenceSession (model_2)");
      const model2 = await ort.InferenceSession.create(MODEL_2_URL, {
        executionProviders: ["wasm"],
      });
      console.log("[dtln] models loaded");
      return { model1, model2 };
    } catch (err) {
      console.warn("[dtln] models not available, running in passthrough mode:", err);
      return null;
    }
  })();
  modelsPromise = p;
  const result = await p;
  if (result === null && modelsPromise === p) {
    modelsPromise = null;
  }
  return result;
}

export async function buildDfnPipeline({
  source,
  context,
}: BuildOptions): Promise<DfnPipeline> {
  console.log("[dtln] adding audio worklet");
  const workletRes = await fetch(assetUrl("worklets/dfn-processor.js"));
  const workletSrc = await workletRes.text();
  const workletBlob = new Blob([workletSrc], { type: "application/javascript" });
  const workletUrl = URL.createObjectURL(workletBlob);
  try {
    await context.audioWorklet.addModule(workletUrl);
  } finally {
    URL.revokeObjectURL(workletUrl);
  }
  console.log("[dtln] worklet added");
  const node = new AudioWorkletNode(context, "dfn-processor");
  const destination = context.createMediaStreamDestination();
  source.connect(node);
  node.connect(destination);

  const models = await loadModels();
  const session = models ? new DtlnSession(models.model1, models.model2) : null;
  node.port.postMessage({ type: "bypass", value: !session });

  const MAX_PENDING = 8;
  let chain: Promise<void> = Promise.resolve();
  let pending = 0;
  let disposed = false;
  node.port.onmessage = (event) => {
    if (disposed) return;
    const msg = event.data;
    if (!msg || msg.type !== "frame" || !(msg.payload instanceof Float32Array)) {
      return;
    }
    if (!session) {
      const passthrough = new Float32Array(HOP);
      passthrough.set(msg.payload);
      node.port.postMessage({ type: "frame", payload: passthrough }, [
        passthrough.buffer,
      ]);
      return;
    }
    if (pending >= MAX_PENDING) {
      node.port.postMessage({ type: "drop" });
      return;
    }
    const payload = msg.payload;
    pending += 1;
    chain = chain.then(async () => {
      if (disposed) {
        pending = Math.max(0, pending - 1);
        return;
      }
      try {
        const processed = await session.process(payload);
        if (!disposed) {
          node.port.postMessage({ type: "frame", payload: processed }, [
            processed.buffer,
          ]);
        }
      } catch (err) {
        console.error("[dtln] inference failed:", err);
        if (!disposed) {
          const fallback = new Float32Array(HOP);
          fallback.set(payload);
          node.port.postMessage({ type: "frame", payload: fallback }, [
            fallback.buffer,
          ]);
        }
      } finally {
        pending = Math.max(0, pending - 1);
      }
    });
  };

  const outputTrack = destination.stream.getAudioTracks()[0];

  return {
    destination,
    outputTrack,
    setStrength: (value: number) => {
      const clamped = Math.min(1, Math.max(0, value));
      node.port.postMessage({ type: "strength", value: clamped });
    },
    dispose: async () => {
      disposed = true;
      try {
        await Promise.race([
          chain,
          new Promise((r) => setTimeout(r, 500)),
        ]);
      } catch {}
      try {
        node.port.onmessage = null;
        node.disconnect();
        source.disconnect(node);
      } catch {}
    },
  };
}
