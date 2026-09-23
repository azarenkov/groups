import {
  RemoteAudioGraph,
  SPEAKING_HOLD_MS,
  SPEAKING_INTERVAL_MS,
  type SpeakingState,
} from "../remoteAudio";

const last = <T,>(items: T[]): T => items[items.length - 1];

interface FakeGain {
  gain: { value: number };
  connect: jest.Mock;
  disconnect: jest.Mock;
}

const created: FakeGain[] = [];
const levels = new Map<string, number>();
let attaching = "";
let failGain = false;
const streamSources: MediaStreamTrack[][] = [];

class FakeMediaStream {
  constructor(readonly tracks: MediaStreamTrack[]) {}
  getAudioTracks() {
    return this.tracks;
  }
}

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
  createMediaStreamSource(stream: FakeMediaStream) {
    streamSources.push(stream.getAudioTracks());
    return { connect: jest.fn(), disconnect: jest.fn() };
  }
  createGain(): FakeGain {
    if (failGain) throw new Error("gain unavailable");
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

interface Attached {
  gain: FakeGain;
  element: HTMLMediaElement;
  end: () => void;
}

const attached: Attached[] = [];

const attach = (graph: RemoteAudioGraph, identity: string): FakeGain => {
  attaching = identity;
  const element = document.createElement("audio");
  const handlers: (() => void)[] = [];
  const track = {
    attach: () => element,
    on: (_event: string, fn: () => void) => handlers.push(fn),
    attachedElements: [element],
    mediaStreamTrack: { id: `${identity}-mic`, kind: "audio" },
  };
  graph.attach(track as never, identity, () => {});
  const gain = created[created.length - 1];
  attached.push({
    gain,
    element,
    end: () => handlers.forEach((fn) => fn()),
  });
  return gain;
};

const lastAttachment = (): Attached => attached[attached.length - 1];

describe("RemoteAudioGraph", () => {
  beforeEach(() => {
    created.length = 0;
    attached.length = 0;
    levels.clear();
    failGain = false;
    streamSources.length = 0;
    (window as unknown as { MediaStream: unknown }).MediaStream =
      FakeMediaStream;
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

  it("tells every subscriber when a participant's volume changes", () => {
    const graph = new RemoteAudioGraph(() => null);
    const heard: [string, number][] = [];
    const stop = graph.subscribeVolume((identity, volume) =>
      heard.push([identity, volume]),
    );

    graph.setVolume("alice", 0.6);
    graph.setVolume("bob", 9);
    stop();
    graph.setVolume("alice", 0.2);

    expect(heard).toEqual([
      ["alice", 0.6],
      ["bob", RemoteAudioGraph.GAIN_MAX],
    ]);
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
    const seen: SpeakingState[] = [];
    graph.subscribeSpeaking((state) => seen.push(state));
    attach(graph, "ivy");
    attach(graph, "jack");

    levels.set("ivy", 0.2);
    jest.advanceTimersByTime(SPEAKING_INTERVAL_MS);
    expect(Array.from(last(seen).speaking)).toEqual(["ivy"]);

    levels.set("ivy", 0);
    jest.advanceTimersByTime(SPEAKING_INTERVAL_MS);
    expect(last(seen).speaking.has("ivy")).toBe(true);

    jest.advanceTimersByTime(SPEAKING_HOLD_MS);
    expect(last(seen).speaking.size).toBe(0);
  });

  it("tells subscribers which participants it measures", () => {
    const graph = new RemoteAudioGraph(() => null);
    const seen: SpeakingState[] = [];
    graph.subscribeSpeaking((state) => seen.push(state));
    expect(last(seen).tracked.size).toBe(0);

    attach(graph, "liam");
    expect(Array.from(last(seen).tracked)).toEqual(["liam"]);

    graph.disableGraph();
    expect(last(seen).tracked.size).toBe(0);
  });

  it("keeps measuring a participant whose second track ended", () => {
    const graph = new RemoteAudioGraph(() => null);
    const seen: SpeakingState[] = [];
    graph.subscribeSpeaking((state) => seen.push(state));
    attach(graph, "mia");
    attach(graph, "mia");
    const second = lastAttachment();

    second.end();
    expect(Array.from(last(seen).tracked)).toEqual(["mia"]);

    levels.set("mia", 0.2);
    jest.advanceTimersByTime(SPEAKING_INTERVAL_MS);
    expect(last(seen).speaking.has("mia")).toBe(true);
  });

  it("keeps measuring the working track when another one fails to wire", () => {
    const graph = new RemoteAudioGraph(() => null);
    const seen: SpeakingState[] = [];
    graph.subscribeSpeaking((state) => seen.push(state));
    attach(graph, "nina");

    failGain = true;
    attach(graph, "nina");
    failGain = false;

    levels.set("nina", 0.2);
    jest.advanceTimersByTime(SPEAKING_INTERVAL_MS);
    expect(last(seen).speaking.has("nina")).toBe(true);
  });

  it("does not claim a participant whose other track fell back", () => {
    const graph = new RemoteAudioGraph(() => null);
    const seen: SpeakingState[] = [];
    graph.subscribeSpeaking((state) => seen.push(state));
    attach(graph, "omar");
    expect(Array.from(last(seen).tracked)).toEqual(["omar"]);

    failGain = true;
    attach(graph, "omar");
    failGain = false;
    const fallback = lastAttachment();
    expect(last(seen).tracked.size).toBe(0);

    fallback.end();
    expect(Array.from(last(seen).tracked)).toEqual(["omar"]);
  });

  it("stops measuring for a participant whose audio fell back", () => {
    const graph = new RemoteAudioGraph(() => null);
    attach(graph, "kate");
    expect(graph.tracksSpeaking()).toBe(true);

    graph.disableGraph();
    expect(graph.tracksSpeaking()).toBe(false);
  });
});

describe("routing", () => {
  beforeEach(() => {
    created.length = 0;
    attached.length = 0;
    failGain = false;
    streamSources.length = 0;
    (window as unknown as { AudioContext: unknown }).AudioContext =
      FakeAudioContext;
    (window as unknown as { MediaStream: unknown }).MediaStream =
      FakeMediaStream;
  });

  it("feeds the gain from the track itself, which chrome delivers for remote webrtc audio", () => {
    const graph = new RemoteAudioGraph(() => null);
    attach(graph, "alice");

    expect(streamSources).toHaveLength(1);
    expect(streamSources[0][0]).toMatchObject({ id: "alice-mic" });
  });

  it("silences the element so the voice is heard only through the gain", () => {
    const graph = new RemoteAudioGraph(() => null);
    attach(graph, "alice");

    expect(lastAttachment().element.muted).toBe(true);
  });

  it("silences the element again when something unmutes it", () => {
    const graph = new RemoteAudioGraph(() => null);
    attach(graph, "alice");
    const { element } = lastAttachment();

    element.muted = false;
    element.dispatchEvent(new Event("volumechange"));

    expect(element.muted).toBe(true);
  });

  it("leaves a fallback element audible", () => {
    const graph = new RemoteAudioGraph(() => null);
    failGain = true;
    attach(graph, "alice");
    failGain = false;

    expect(lastAttachment().element.muted).toBe(false);
  });
});

describe("ownership", () => {
  it("knows which elements it plays", () => {
    const graph = new RemoteAudioGraph(() => null);
    attach(graph, "rosa");
    const own = document.querySelectorAll("audio");
    expect(graph.manages(own[own.length - 1] as HTMLMediaElement)).toBe(true);
    expect(graph.manages(document.createElement("audio"))).toBe(false);
  });
});

describe("deafening", () => {
  it("silences a participant's screen audio as well as their microphone", () => {
    const graph = new RemoteAudioGraph(() => null);
    const mic = attach(graph, "pete");
    const screen = attach(graph, "pete");

    graph.setVolume("pete", 1.5);
    graph.setDeafened(true);
    expect(mic.gain.value).toBe(0);
    expect(screen.gain.value).toBe(0);

    graph.setDeafened(false);
    expect(mic.gain.value).toBeCloseTo(1.5);
    expect(screen.gain.value).toBeCloseTo(1.5);
  });

  it("silences a fallback track next to a wired one", () => {
    const graph = new RemoteAudioGraph(() => null);
    const wired = attach(graph, "sam");
    failGain = true;
    attach(graph, "sam");
    failGain = false;
    const fallback = lastAttachment();

    graph.setVolume("sam", 0.5);
    expect(wired.gain.value).toBeCloseTo(0.5);
    expect(fallback.element.volume).toBeCloseTo(0.5);

    graph.setDeafened(true);
    expect(wired.gain.value).toBe(0);
    expect(fallback.element.volume).toBe(0);

    graph.setDeafened(false);
    expect(fallback.element.volume).toBeCloseTo(0.5);
  });

  it("silences audio that arrives while deafened", () => {
    const graph = new RemoteAudioGraph(() => null);
    graph.setDeafened(true);
    const late = attach(graph, "quinn");

    expect(late.gain.value).toBe(0);

    graph.setDeafened(false);
    expect(late.gain.value).toBeCloseTo(1);
  });
});
