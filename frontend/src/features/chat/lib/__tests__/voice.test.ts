import {
  pickVideoNoteMimeType,
  videoNoteDurationFromPayload,
  downsampleWaveform,
  formatVoiceDuration,
  isVoiceMessage,
  pickVoiceMimeType,
  voiceFileName,
} from "../voice";

describe("voice helpers", () => {
  it("picks the first recorder format the browser supports", () => {
    expect(pickVoiceMimeType((t) => t.startsWith("audio/webm"))).toBe("audio/webm;codecs=opus");
    expect(pickVoiceMimeType((t) => t === "audio/mp4")).toBe("audio/mp4");
    expect(pickVoiceMimeType(() => false)).toBeNull();
  });

  it("names the file after its container", () => {
    expect(voiceFileName("audio/webm;codecs=opus")).toBe("voice.webm");
    expect(voiceFileName("audio/mp4")).toBe("voice.m4a");
    expect(voiceFileName("audio/ogg;codecs=opus")).toBe("voice.ogg");
  });

  it("formats durations like a player", () => {
    expect(formatVoiceDuration(0)).toBe("0:00");
    expect(formatVoiceDuration(42_400)).toBe("0:42");
    expect(formatVoiceDuration(65_000)).toBe("1:05");
    expect(formatVoiceDuration(3_725_000)).toBe("1:02:05");
  });

  it("squeezes levels into bars scaled to the loudest one", () => {
    const bars = downsampleWaveform([0, 0.5, 1, 0.25], 2);
    expect(bars).toEqual([128, 255]);
    expect(downsampleWaveform([], 4)).toEqual([0, 0, 0, 0]);
    expect(downsampleWaveform([0.1, 0.2], 4)).toHaveLength(4);
  });

  it("recognises a voice message by its attachment", () => {
    expect(isVoiceMessage({ attachments: [] })).toBe(false);
    expect(
      isVoiceMessage({
        attachments: [
          { id: "a", fileName: "voice.webm", contentType: "audio/webm", sizeBytes: 1, voice: { durationMs: 1, waveform: [] } },
        ],
      }),
    ).toBe(true);
  });

  it("picks a video note format the browser can record", () => {
    expect(pickVideoNoteMimeType((t) => t.startsWith("video/webm;codecs=vp8"))).toBe("video/webm;codecs=vp8,opus");
    expect(pickVideoNoteMimeType((t) => t === "video/mp4")).toBe("video/mp4");
    expect(pickVideoNoteMimeType(() => false)).toBeNull();
  });

  it("reads a video note length from an event payload", () => {
    expect(videoNoteDurationFromPayload([{ videoNote: { durationMs: 17_000 } }])).toBe(17_000);
    expect(videoNoteDurationFromPayload([{ voice: { durationMs: 1 } }])).toBeNull();
    expect(videoNoteDurationFromPayload(null)).toBeNull();
  });

  it("treats a video note as a single-media message", () => {
    expect(
      isVoiceMessage({
        attachments: [
          { id: "v", fileName: "v.webm", contentType: "video/webm", sizeBytes: 1, videoNote: { durationMs: 1, size: 384, poster: "" } },
        ],
      }),
    ).toBe(true);
  });
});
