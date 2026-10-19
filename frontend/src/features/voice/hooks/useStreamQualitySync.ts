import React from "react";

import { getLiveKitService } from "../lib/livekit";
import { resolveStreamQuality } from "../lib/streamQuality";
import { useVoiceSettingsStore } from "../store/voiceSettingsStore";

export function useStreamQualitySync() {
  const wanted = useVoiceSettingsStore((s) => s.streamQuality);
  React.useEffect(() => {
    const quality = resolveStreamQuality(wanted);
    void getLiveKitService().setScreenShareQuality({
      width: quality.width,
      height: quality.height,
      frameRate: quality.frameRate,
      maxBitrate: quality.maxBitrate,
      adaptive: wanted === "auto",
    });
  }, [wanted]);
}
