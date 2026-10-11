import { MESSAGE_CHIME, playChime } from "../chime";

class FakeNode {
  connections = new Set<FakeNode>();
  gain = { value: 1, setValueAtTime: jest.fn(), exponentialRampToValueAtTime: jest.fn() };
  frequency = { value: 0, setValueAtTime: jest.fn() };
  delayTime = { value: 0 };
  Q = { value: 0 };
  type = "";
  connect(node: FakeNode) {
    this.connections.add(node);
    return node;
  }
  disconnect() {
    this.connections.clear();
  }
  start() {}
  stop() {}
}

it("disconnects the whole chime graph once the sound and its echo have faded", () => {
  jest.useFakeTimers();
  const nodes: FakeNode[] = [];
  const make = () => {
    const node = new FakeNode();
    nodes.push(node);
    return node;
  };
  const destination = new FakeNode();
  const ac = {
    currentTime: 0,
    destination,
    createGain: make,
    createBiquadFilter: make,
    createDelay: make,
    createOscillator: make,
  } as unknown as BaseAudioContext;
  playChime(ac, MESSAGE_CHIME, 0.1);
  const master = nodes[0];
  expect(master.connections.has(destination)).toBe(true);
  jest.advanceTimersByTime(5000);
  expect(master.connections.size).toBe(0);
  expect(nodes.slice(0, 7).every((n) => n.connections.size === 0)).toBe(true);
  jest.useRealTimers();
});
