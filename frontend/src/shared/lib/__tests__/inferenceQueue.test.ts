export interface QueueEvent {
  seq: number;
  action: "process" | "drop";
  reset: boolean;
}

export function planQueue(
  sequences: number[],
  capacity: number,
  warmupFrames: number,
): QueueEvent[] {
  const events: QueueEvent[] = [];
  let pending = 0;
  let lastProcessed: number | null = null;
  let warmup = 0;
  const queue: number[] = [];

  const run = () => {
    while (queue.length > 0) {
      const seq = queue.shift() as number;
      const reset = lastProcessed !== null && seq !== lastProcessed + 1;
      if (reset) warmup = warmupFrames;
      lastProcessed = seq;
      if (warmup > 0) {
        warmup -= 1;
        events.push({ seq, action: "drop", reset });
      } else {
        events.push({ seq, action: "process", reset });
      }
      pending -= 1;
    }
  };

  for (const seq of sequences) {
    if (pending >= capacity) {
      events.push({ seq, action: "drop", reset: false });
      continue;
    }
    pending += 1;
    queue.push(seq);
  }
  run();
  return events;
}

describe("the inference queue", () => {
  it("accepts a steady stream without resetting", () => {
    const events = planQueue([0, 1, 2, 3], 8, 3);
    expect(events.every((e) => !e.reset)).toBe(true);
    expect(events.filter((e) => e.action === "process")).toHaveLength(4);
  });

  it("drops what does not fit", () => {
    const events = planQueue([0, 1, 2], 2, 3);
    expect(events.filter((e) => e.action === "drop").map((e) => e.seq)).toEqual([2]);
  });

  it("resets on the frame that follows the gap, not on a queued one", () => {
    const events = planQueue([0, 1, 2, 4], 8, 3);
    const reset = events.filter((e) => e.reset).map((e) => e.seq);
    expect(reset).toEqual([4]);
  });

  it("plays dry until the model window is full again", () => {
    const events = planQueue([0, 4, 5, 6, 7, 8], 8, 3);
    const dry = events.filter((e) => e.action === "drop").map((e) => e.seq);
    expect(dry).toEqual([4, 5, 6]);
    expect(events[events.length - 1]).toEqual({
      seq: 8,
      action: "process",
      reset: false,
    });
  });
});
