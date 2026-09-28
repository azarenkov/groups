import {
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
});
