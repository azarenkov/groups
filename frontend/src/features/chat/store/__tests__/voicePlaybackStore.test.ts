import { nextAutoplayAfter, registerVoicePlayer, useVoicePlaybackStore } from "../voicePlaybackStore";

const node = () => {
  const el = document.createElement("div");
  document.body.appendChild(el);
  return el;
};

describe("voice playback", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("finds the next unheard voice note below the current one", () => {
    const [a, b, c, d] = [node(), node(), node(), node()];
    const played: string[] = [];
    const handle = (el: HTMLElement, id: string, autoplayable: boolean) =>
      registerVoicePlayer(id, { element: () => el, autoplayable: () => autoplayable, play: () => played.push(id) });
    const off = [handle(a, "a", true), handle(b, "b", false), handle(d, "d", true), handle(c, "c", true)];
    nextAutoplayAfter("a")?.play();
    nextAutoplayAfter("c")?.play();
    expect(played).toEqual(["c", "d"]);
    expect(nextAutoplayAfter("d")).toBeNull();
    off.forEach((f) => f());
  });

  it("cycles and remembers the speed", () => {
    const store = useVoicePlaybackStore.getState();
    store.cycleSpeed();
    expect(useVoicePlaybackStore.getState().speed).toBe(1.5);
    expect(localStorage.getItem("rift.voiceSpeed")).toBe("1.5");
    useVoicePlaybackStore.getState().cycleSpeed();
    useVoicePlaybackStore.getState().cycleSpeed();
    expect(useVoicePlaybackStore.getState().speed).toBe(1);
  });
});
