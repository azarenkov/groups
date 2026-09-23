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

const serve = (port: MessagePort) => {
  let chain: Promise<void> = Promise.resolve();
  port.onmessage = (event: MessageEvent) => {
    const msg = event.data;
    if (!msg || msg.type !== "frame") return;
    const seq: number = msg.seq;
    const payload = msg.payload as Float32Array;
    if (!session) {
      port.postMessage({ type: "frame", seq, payload }, [payload.buffer]);
      return;
    }
    chain = chain.then(async () => {
      try {
        const processed = await session!.process(payload);
        port.postMessage({ type: "frame", seq, payload: processed }, [
          processed.buffer,
        ]);
      } catch {
        port.postMessage({ type: "drop", seq });
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
