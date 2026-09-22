import {
  RemoteAudioGraph,
  SPEAKING_HOLD_MS,
  SPEAKING_INTERVAL_MS,
} from "../remoteAudio";

interface FakeGain {
  gain: { value: number };
  connect: jest.Mock;
  disconnect: jest.Mock;
}

const created: FakeGain[] = [];
const levels = new Map<string, number>();
let attaching = "";

class FakeAudioContext {
  createAnalyser() {
    const identity = attaching;
    return {
      fftSize: 512,
      frequencyBinCount: 256,
      connect: jest.fn(),
      disconnect: jest.fn(),
      getFloatTimeDomainData: (buffer: Float32Array) => {
        buffer.fill(levels.get(identity) ?? 0);
      },
    };
  }

  state = "running";
  destination = {};
  createMediaElementSource() {
    return { connect: jest.fn(), disconnect: jest.fn() };
  }
  createGain(): FakeGain {
    const gain: FakeGain = {
      gain: { value: 1 },
      connect: jest.fn(),
      disconnect: jest.fn(),
    };
    created.push(gain);
    return gain;
  }
  createDynamicsCompressor() {
    return {
      threshold: { value: 0 },
      knee: { value: 0 },
      ratio: { value: 0 },
      attack: { value: 0 },
      release: { value: 0 },
      connect: jest.fn(),
    };
  }
  resume() {
    return Promise.resolve();
  }
}

const attach = (graph: RemoteAudioGraph, identity: string): FakeGain => {
  attaching = identity;
  const element = document.createElement("audio");
  const track = {
    attach: () => element,
    on: jest.fn(),
    attachedElements: [element],
  };
  graph.attach(track as never, identity, () => {});
  return created[created.length - 1];
};

describe("RemoteAudioGraph", () => {
  beforeEach(() => {
    created.length = 0;
    levels.clear();
    jest.useFakeTimers();
    (window as unknown as { AudioContext: unknown }).AudioContext =
      FakeAudioContext;
  });

  afterEach(() => {
    jest.clearAllTimers();
    jest.useRealTimers();
  });

  it("applies a volume to the gain of that participant only", () => {
    const graph = new RemoteAudioGraph(() => null);
    const alice = attach(graph, "alice");
    const bob = attach(graph, "bob");

    graph.setVolume("alice", 0.4);

    expect(alice.gain.value).toBeCloseTo(0.4);
    expect(bob.gain.value).toBeCloseTo(1);
    expect(graph.getVolume("alice")).toBeCloseTo(0.4);
    expect(graph.getVolume("bob")).toBeCloseTo(1);
  });

  it("keeps a volume set before the track arrives", () => {
    const graph = new RemoteAudioGraph(() => null);
    graph.setVolume("carol", 1.8);

    expect(attach(graph, "carol").gain.value).toBeCloseTo(1.8);
  });

  it("scales participant volume by the master volume", () => {
    const graph = new RemoteAudioGraph(() => null);
    const dave = attach(graph, "dave");

    graph.setVolume("dave", 2);
    graph.setMasterVolume(0.5);

    expect(dave.gain.value).toBeCloseTo(1);
  });

  it("silences everyone while deafened and restores the volume afterwards", () => {
    const graph = new RemoteAudioGraph(() => null);
    const erin = attach(graph, "erin");

    graph.setVolume("erin", 1.5);
    graph.setDeafened(true);
    expect(erin.gain.value).toBe(0);

    graph.setDeafened(false);
    expect(erin.gain.value).toBeCloseTo(1.5);
  });

  it("clamps the volume to the supported range", () => {
    const graph = new RemoteAudioGraph(() => null);
    const frank = attach(graph, "frank");

    graph.setVolume("frank", 99);
    expect(frank.gain.value).toBeCloseTo(RemoteAudioGraph.GAIN_MAX);

    graph.setVolume("frank", -1);
    expect(frank.gain.value).toBe(0);
  });

  it("reports boost support per participant", () => {
    const graph = new RemoteAudioGraph(() => null);
    expect(graph.supportsBoost("grace")).toBe(true);

    attach(graph, "grace");
    expect(graph.supportsBoost("grace")).toBe(true);
  });

  it("denies boost for a participant that fell back to element volume", () => {
    const graph = new RemoteAudioGraph(() => null);
    (window as unknown as { AudioContext: unknown }).AudioContext = class {
      state = "running";
      destination = {};
      createMediaElementSource() {
        throw new Error("already connected");
      }
      resume() {
        return Promise.resolve();
      }
    };

    attach(graph, "heidi");

    expect(graph.supportsBoost("heidi")).toBe(false);
  });

  it("reports who is talking from the measured level", () => {
    const graph = new RemoteAudioGraph(() => null);
    const seen: Set<string>[] = [];
    graph.subscribeSpeaking((ids) => seen.push(ids));
    attach(graph, "ivy");
    attach(graph, "jack");

    levels.set("ivy", 0.2);
    jest.advanceTimersByTime(SPEAKING_INTERVAL_MS);
    expect(Array.from(seen[seen.length - 1])).toEqual(["ivy"]);

    levels.set("ivy", 0);
    jest.advanceTimersByTime(SPEAKING_INTERVAL_MS);
    expect(seen[seen.length - 1].has("ivy")).toBe(true);

    jest.advanceTimersByTime(SPEAKING_HOLD_MS);
    expect(seen[seen.length - 1].size).toBe(0);
  });

  it("stops measuring for a participant whose audio fell back", () => {
    const graph = new RemoteAudioGraph(() => null);
    attach(graph, "kate");
    expect(graph.tracksSpeaking()).toBe(true);

    graph.disableGraph();
    expect(graph.tracksSpeaking()).toBe(false);
  });
});
