export type VideoCodec = "av1" | "vp9";
export type ScreenContent = "auto" | "text" | "motion";

export const CPU_LIMIT_HOLD_MS = 10000;
export const CPU_LIMIT_FPS_RATIO = 0.5;

export interface EncodingProbe {
  supported: boolean;
  smooth: boolean;
  powerEfficient: boolean;
}

export function chooseCodec(
  desktop: boolean,
  probe: EncodingProbe | null,
): VideoCodec {
  if (!desktop) return "vp9";
  if (!probe) return "vp9";
  return probe.supported && probe.smooth && probe.powerEfficient ? "av1" : "vp9";
}

export type Degradation =
  | "balanced"
  | "maintain-resolution"
  | "maintain-framerate";

export function degradationFor(
  content: ScreenContent,
  adaptive: boolean,
): Degradation {
  if (content === "text") return "maintain-resolution";
  if (content === "motion") return "maintain-framerate";
  return adaptive ? "balanced" : "maintain-resolution";
}

export interface EncoderSample {
  cpuLimited: boolean;
  framesPerSecond: number;
  sourceFramesPerSecond: number | null;
  at: number;
}

export function isStruggling(sample: EncoderSample): boolean {
  if (sample.cpuLimited) return true;
  if (sample.sourceFramesPerSecond === null) return false;
  return (
    sample.framesPerSecond < sample.sourceFramesPerSecond * CPU_LIMIT_FPS_RATIO
  );
}

export function cpuLimitedSince(samples: EncoderSample[]): number | null {
  let since: number | null = null;
  for (const sample of samples) {
    const struggling = isStruggling(sample);
    if (!struggling) {
      since = null;
      continue;
    }
    if (since === null) since = sample.at;
  }
  return since;
}

export function shouldFallBack(samples: EncoderSample[], now: number): boolean {
  const since = cpuLimitedSince(samples);
  if (since === null) return false;
  return now - since >= CPU_LIMIT_HOLD_MS;
}

export function canEncode(codec: VideoCodec): boolean {
  if (typeof RTCRtpSender === "undefined" || !RTCRtpSender.getCapabilities) {
    return false;
  }
  const mime = codec === "av1" ? "video/av1" : "video/vp9";
  const caps = RTCRtpSender.getCapabilities("video");
  return !!caps?.codecs.some((c) => c.mimeType.toLowerCase() === mime);
}

export async function probeEncoding(
  codec: VideoCodec,
  width: number,
  height: number,
  frameRate: number,
  bitrate: number,
): Promise<EncodingProbe | null> {
  const capabilities = (
    navigator as unknown as {
      mediaCapabilities?: {
        encodingInfo?: (config: unknown) => Promise<EncodingProbe>;
      };
    }
  ).mediaCapabilities;
  if (!capabilities?.encodingInfo) return null;
  try {
    const info = await capabilities.encodingInfo({
      type: "webrtc",
      video: {
        contentType: codec === "av1" ? "video/AV1" : 'video/VP9; profile-id="0"',
        width,
        height,
        framerate: frameRate,
        bitrate,
      },
    });
    return {
      supported: !!info.supported,
      smooth: !!info.smooth,
      powerEfficient: !!info.powerEfficient,
    };
  } catch {
    return null;
  }
}

export async function pickScreenCodec(
  width: number,
  height: number,
  frameRate: number,
  bitrate: number,
): Promise<VideoCodec> {
  const desktop =
    typeof window !== "undefined" && !!(window as { electron?: unknown }).electron;
  if (!desktop || !canEncode("av1")) return "vp9";
  const probe = await probeEncoding("av1", width, height, frameRate, bitrate);
  return chooseCodec(desktop, probe);
}
