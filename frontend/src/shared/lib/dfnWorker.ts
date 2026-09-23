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

const serve = (port: MessagePort) => {
  let chain: Promise<void> = Promise.resolve();
  let pending = 0;
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
        const processed = await session!.process(payload);
        port.postMessage({ type: "frame", seq, payload: processed }, [
          processed.buffer,
        ]);
      } catch {
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
