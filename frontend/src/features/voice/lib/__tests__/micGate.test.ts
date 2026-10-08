import {
  MIC_GATE_ATTACK_S,
  MIC_GATE_LOOKAHEAD_S,
  MIC_GATE_RELEASE_S,
  MIC_GATE_TICK_MS,
  connectThroughLookahead,
  isGateOpen,
  setGateOpen,
} from "../micGate";

const fakeGate = () => {
  const gain = { value: 1, cancelScheduledValues: jest.fn(), setTargetAtTime: jest.fn() };
  return { gain, context: { currentTime: 5 } } as unknown as GainNode & { gain: typeof gain };
};

it("delays the voice longer than it takes the gate to notice speech", () => {
  expect(MIC_GATE_LOOKAHEAD_S * 1000).toBeGreaterThan(MIC_GATE_TICK_MS * 2);
  expect(MIC_GATE_LOOKAHEAD_S).toBeLessThanOrEqual(0.1);
});

it("routes audio through a delay before the gate", () => {
  const delay = { delayTime: { value: 0 }, connect: jest.fn() };
  const ctx = { createDelay: jest.fn(() => delay) } as unknown as BaseAudioContext;
  const from = { connect: jest.fn() } as unknown as AudioNode;
  const gate = fakeGate();
  connectThroughLookahead(ctx, from, gate);
  expect(delay.delayTime.value).toBe(MIC_GATE_LOOKAHEAD_S);
  expect((from.connect as jest.Mock).mock.calls[0][0]).toBe(delay);
  expect(delay.connect).toHaveBeenCalledWith(gate);
});

it("fades the gate in fast and out gently, only when its state changes", () => {
  const gate = fakeGate();
  setGateOpen(gate, false);
  expect(gate.gain.setTargetAtTime).toHaveBeenLastCalledWith(0, 5, MIC_GATE_RELEASE_S);
  expect(isGateOpen(gate)).toBe(false);
  setGateOpen(gate, false);
  expect(gate.gain.setTargetAtTime).toHaveBeenCalledTimes(1);
  setGateOpen(gate, true);
  expect(gate.gain.setTargetAtTime).toHaveBeenLastCalledWith(1, 5, MIC_GATE_ATTACK_S);
  expect(isGateOpen(gate)).toBe(true);
});
