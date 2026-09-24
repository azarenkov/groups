import initDeepFilter, { Denoiser } from "./deepfilter/deepfilter.js";
import { attenuationFor } from "./denoiserStrength";

interface InitMessage {
  type: "init";
  model: string;
  port: MessagePort;
}

interface StrengthMessage {
  type: "strength";
  value: number;
}

const WASM_URL = new URL("./deepfilter/deepfilter_bg.wasm", import.meta.url);

let denoiser: Denoiser | null = null;

const readBytes = async (url: string): Promise<ArrayBuffer> => {
  const response = await fetch(url);
  const local = response.status === 0 && url.startsWith("file:");
  if (!response.ok && !local) {
    throw new Error(`${url} request failed: ${response.status}`);
  }
  return response.arrayBuffer();
};

const loadDenoiser = async (modelUrl: string): Promise<Denoiser | null> => {
  try {
    const [model, runtime] = await Promise.all([
      readBytes(modelUrl),
      readBytes(WASM_URL.toString()),
    ]);
    await initDeepFilter({ module_or_path: runtime });
    return new Denoiser(new Uint8Array(model), attenuationFor(1));
  } catch (e) {
    console.warn("[dfnWorker] DeepFilterNet unavailable:", e);
    return null;
  }
};

const serve = (port: MessagePort, model: Denoiser) => {
  const frameLength = model.frameLength;
  port.onmessage = (event: MessageEvent) => {
    const msg = event.data;
    if (!msg || msg.type !== "frame") return;
    const seq: number = msg.seq;
    const payload = msg.payload as Float32Array;
    if (payload.length !== frameLength) {
      port.postMessage({ type: "drop", seq });
      return;
    }
    const enhanced = new Float32Array(frameLength);
    try {
      model.process(payload, enhanced);
    } catch {
      port.postMessage({ type: "drop", seq });
      return;
    }
    port.postMessage({ type: "frame", seq, payload: enhanced }, [
      enhanced.buffer,
    ]);
  };
  port.start();
};

const scope = globalThis as unknown as Worker;

scope.onmessage = async (
  event: MessageEvent<InitMessage | StrengthMessage>,
) => {
  const msg = event.data;
  if (!msg) return;
  if (msg.type === "strength") {
    denoiser?.setAttenuationLimit(attenuationFor(msg.value));
    return;
  }
  if (msg.type !== "init") return;
  denoiser = await loadDenoiser(msg.model);
  if (denoiser) serve(msg.port, denoiser);
  scope.postMessage({
    type: "ready",
    denoising: !!denoiser,
    frameLength: denoiser?.frameLength ?? 0,
    latency: denoiser?.latency ?? 0,
    sampleRate: denoiser?.sampleRate ?? 0,
  });
};

export {};
