import { DECIMATION, LOWPASS_TAPS, designLowpass } from "./bandSplit";
import { DENOISER_SAMPLE_RATE } from "./dtlnSession";

export const PIPELINE_SAMPLE_RATE = DENOISER_SAMPLE_RATE * DECIMATION;
export { DENOISER_SAMPLE_RATE };

const LOWPASS_CUTOFF_HZ = 7200;
const READY_TIMEOUT_MS = 15000;

export interface DfnPipeline {
  destination: MediaStreamAudioDestinationNode;
  outputTrack: MediaStreamTrack;
  setStrength: (value: number) => void;
  dispose: () => Promise<void>;
  hasDenoiser: boolean;
}

interface BuildOptions {
  source: MediaStreamAudioSourceNode;
  context: AudioContext;
}

function assetBase(): string {
  const electron = (window as unknown as { electron?: { appBase?: string } })
    .electron;
  if (electron?.appBase) return electron.appBase;
  const pub = process.env.PUBLIC_URL || ".";
  const baseStr = pub.endsWith("/") ? pub : pub + "/";
  return new URL(baseStr, window.location.origin + "/").toString();
}

function assetUrl(rel: string): string {
  return new URL(rel, assetBase()).toString();
}

async function addWorklet(context: AudioContext): Promise<void> {
  const response = await fetch(assetUrl("worklets/dfn-processor.js"));
  const source = await response.text();
  const blob = new Blob([source], { type: "application/javascript" });
  const url = URL.createObjectURL(blob);
  try {
    await context.audioWorklet.addModule(url);
  } finally {
    URL.revokeObjectURL(url);
  }
}

function startWorker(): Worker {
  return new Worker(new URL("./dfnWorker.ts", import.meta.url));
}

export async function buildDfnPipeline({
  source,
  context,
}: BuildOptions): Promise<DfnPipeline> {
  await addWorklet(context);
  const node = new AudioWorkletNode(context, "dfn-processor");
  const destination = context.createMediaStreamDestination();
  source.connect(node);
  node.connect(destination);

  const worker = startWorker();
  const channel = new MessageChannel();
  const coefficients = designLowpass(
    LOWPASS_TAPS,
    LOWPASS_CUTOFF_HZ,
    context.sampleRate,
  );

  const ready = new Promise<boolean>((resolve) => {
    const timer = setTimeout(() => resolve(false), READY_TIMEOUT_MS);
    worker.onmessage = (event: MessageEvent) => {
      if (!event.data || event.data.type !== "ready") return;
      clearTimeout(timer);
      resolve(!!event.data.denoising);
    };
  });

  node.port.postMessage(
    {
      type: "init",
      coefficients,
      decimation: DECIMATION,
      port: channel.port2,
    },
    [channel.port2],
  );

  worker.postMessage(
    {
      type: "init",
      model1: assetUrl("models/dtln/model_1.onnx"),
      model2: assetUrl("models/dtln/model_2.onnx"),
      wasmPath: assetUrl("ort/"),
      port: channel.port1,
    },
    [channel.port1],
  );

  const hasDenoiser = await ready;
  node.port.postMessage({ type: "bypass", value: !hasDenoiser });

  const outputTrack = destination.stream.getAudioTracks()[0];

  return {
    destination,
    outputTrack,
    hasDenoiser,
    setStrength: (value: number) => {
      const clamped = Math.min(1, Math.max(0, value));
      node.port.postMessage({ type: "strength", value: clamped });
    },
    dispose: async () => {
      try {
        node.port.onmessage = null;
        node.disconnect();
        source.disconnect(node);
      } catch {}
      try {
        worker.terminate();
      } catch {}
    },
  };
}
