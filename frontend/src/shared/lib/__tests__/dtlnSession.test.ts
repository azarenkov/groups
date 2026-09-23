import { DtlnSession, HOP, STATE_SIZE, WINDOW } from "../dtlnSession";

interface FakeResult {
  [key: string]: { data: Float32Array };
}

const fakeModel = (outputLength: number, fill: number) => {
  let calls = 0;
  return {
    inputNames: ["input", "state"],
    outputNames: ["output", "state"],
    run: async (): Promise<FakeResult> => {
      calls += 1;
      return {
        output: { data: new Float32Array(outputLength).fill(fill) },
        state: { data: new Float32Array(STATE_SIZE).fill(calls) },
      };
    },
    get calls() {
      return calls;
    },
  };
};

describe("the model session", () => {
  it("clears its windows and recurrent state on reset", async () => {
    const model1 = fakeModel(WINDOW / 2 + 1, 1);
    const model2 = fakeModel(WINDOW, 0.5);
    const session = new DtlnSession(model1 as never, model2 as never);

    const loud = new Float32Array(HOP).fill(1);
    const first = await session.process(loud);
    expect(first.length).toBe(HOP);

    session.reset();

    const silence = new Float32Array(HOP);
    const after = await session.process(silence);
    const fresh = new DtlnSession(model1 as never, model2 as never);
    const reference = await fresh.process(silence);

    for (let i = 0; i < HOP; i++) {
      expect(after[i]).toBeCloseTo(reference[i], 6);
    }
  });

  it("takes four hops before its output window is complete", async () => {
    const model1 = fakeModel(WINDOW / 2 + 1, 1);
    const model2 = fakeModel(WINDOW, 1);
    const session = new DtlnSession(model1 as never, model2 as never);

    const silence = new Float32Array(HOP);
    const outputs: number[] = [];
    for (let i = 0; i < WINDOW / HOP; i++) {
      const out = await session.process(silence);
      outputs.push(out[0]);
    }

    expect(outputs[0]).toBeCloseTo(1, 6);
    expect(outputs[outputs.length - 1]).toBeCloseTo(WINDOW / HOP, 6);
  });
});
