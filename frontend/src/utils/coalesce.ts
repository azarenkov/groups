export function coalesce(run: () => Promise<void>): () => Promise<void> {
  let inflight: Promise<void> | null = null;
  let queued = false;
  const start = (): Promise<void> =>
    run()
      .catch(() => {})
      .then(() => {
        if (!queued) return;
        queued = false;
        return start();
      });
  return () => {
    if (inflight) {
      queued = true;
      return inflight;
    }
    inflight = start().finally(() => {
      inflight = null;
    });
    return inflight;
  };
}
