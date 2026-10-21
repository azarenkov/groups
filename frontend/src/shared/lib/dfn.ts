import { assetUrl } from "./assets";

export const PIPELINE_SAMPLE_RATE = 48000;

const MODEL_PATH = "models/deepfilter/DeepFilterNet3_onnx.tar.gz";
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

interface ReadyMessage {
  type: "ready";
  denoising: boolean;
  frameLength: number;
  latency: number;
  sampleRate: number;
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

  const ready = new Promise<ReadyMessage | null>((resolve) => {
    const timer = setTimeout(() => resolve(null), READY_TIMEOUT_MS);
    worker.onmessage = (event: MessageEvent) => {
      const msg = event.data as ReadyMessage | undefined;
      if (!msg || msg.type !== "ready") return;
      clearTimeout(timer);
      resolve(msg);
    };
  });

  worker.postMessage(
    { type: "init", model: assetUrl(MODEL_PATH), port: channel.port1 },
    [channel.port1],
  );

  const status = await ready;
  const hasDenoiser =
    !!status?.denoising &&
    status.frameLength > 0 &&
    status.sampleRate === context.sampleRate;

  if (hasDenoiser && status) {
    node.port.postMessage(
      {
        type: "init",
        frameLength: status.frameLength,
        latency: status.latency,
        port: channel.port2,
      },
      [channel.port2],
    );
  } else {
    worker.terminate();
  }

  const outputTrack = destination.stream.getAudioTracks()[0];

  return {
    destination,
    outputTrack,
    hasDenoiser,
    setStrength: (value: number) => {
      if (!hasDenoiser) return;
      worker.postMessage({ type: "strength", value });
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
