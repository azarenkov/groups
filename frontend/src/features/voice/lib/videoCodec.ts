export type CodecPreference = "auto" | "av1" | "vp9";
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
  preference: CodecPreference,
  desktop: boolean,
  probe: EncodingProbe | null,
): VideoCodec {
  if (preference === "vp9") return "vp9";
  if (preference === "av1") return "av1";
  if (!desktop) return "vp9";
  if (!probe) return "vp9";
  return probe.supported && probe.smooth && probe.powerEfficient ? "av1" : "vp9";
}

export function contentHintFor(
  content: ScreenContent,
  frameRate: number,
): "detail" | "motion" {
  if (content === "text") return "detail";
  if (content === "motion") return "motion";
  return frameRate >= 60 ? "motion" : "detail";
}

export interface EncoderSample {
  cpuLimited: boolean;
  framesPerSecond: number;
  at: number;
}

export function cpuLimitedSince(
  samples: EncoderSample[],
  targetFrameRate: number,
): number | null {
  let since: number | null = null;
  for (const sample of samples) {
    const struggling =
      sample.cpuLimited ||
      sample.framesPerSecond < targetFrameRate * CPU_LIMIT_FPS_RATIO;
    if (!struggling) {
      since = null;
      continue;
    }
    if (since === null) since = sample.at;
  }
  return since;
}

export function shouldFallBack(
  samples: EncoderSample[],
  targetFrameRate: number,
  now: number,
): boolean {
  const since = cpuLimitedSince(samples, targetFrameRate);
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
  preference: CodecPreference,
  width: number,
  height: number,
  frameRate: number,
  bitrate: number,
): Promise<VideoCodec> {
  const desktop =
    typeof window !== "undefined" && !!(window as { electron?: unknown }).electron;
  if (preference === "av1" && !canEncode("av1")) return "vp9";
  if (preference !== "auto") return chooseCodec(preference, desktop, null);
  if (!desktop || !canEncode("av1")) return "vp9";
  const probe = await probeEncoding("av1", width, height, frameRate, bitrate);
  return chooseCodec("auto", desktop, probe);
}
