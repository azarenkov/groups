import { configureRuntime, loadSession } from "./dtlnSession";
import type { DtlnSession } from "./dtlnSession";

interface InitMessage {
  type: "init";
  model1: string;
  model2: string;
  wasmPath: string;
  port: MessagePort;
}

let session: DtlnSession | null = null;

const MAX_PENDING = 8;
const WARMUP_FRAMES = 3;

const serve = (port: MessagePort) => {
  let chain: Promise<void> = Promise.resolve();
  let pending = 0;
  let lastProcessed: number | null = null;
  let warmup = 0;
  port.onmessage = (event: MessageEvent) => {
    const msg = event.data;
    if (!msg || msg.type !== "frame") return;
    const seq: number = msg.seq;
    const payload = msg.payload as Float32Array;
    if (!session) {
      port.postMessage({ type: "frame", seq, payload }, [payload.buffer]);
      return;
    }
    if (pending >= MAX_PENDING) {
      port.postMessage({ type: "drop", seq });
      return;
    }
    pending += 1;
    chain = chain.then(async () => {
      try {
        if (lastProcessed !== null && seq !== lastProcessed + 1) {
          session!.reset();
          warmup = WARMUP_FRAMES;
        }
        const processed = await session!.process(payload);
        lastProcessed = seq;
        if (warmup > 0) {
          warmup -= 1;
          port.postMessage({ type: "drop", seq });
          return;
        }
        port.postMessage({ type: "frame", seq, payload: processed }, [
          processed.buffer,
        ]);
      } catch {
        session!.reset();
        warmup = WARMUP_FRAMES;
        lastProcessed = seq;
        port.postMessage({ type: "drop", seq });
      } finally {
        pending = Math.max(0, pending - 1);
      }
    });
  };
  port.start();
};

const scope = globalThis as unknown as Worker;

scope.onmessage = async (event: MessageEvent<InitMessage>) => {
  const msg = event.data;
  if (!msg || msg.type !== "init") return;
  configureRuntime(msg.wasmPath);
  session = await loadSession(msg.model1, msg.model2);
  serve(msg.port);
  scope.postMessage({
    type: "ready",
    denoising: !!session,
  });
};

export {};
