import React from "react";

import { getLiveKitService } from "../../voice/lib/livekit";
import { useVoiceSettingsStore } from "../../voice/store/voiceSettingsStore";
import { bestAllowedQuality } from "../lib/plans";
import { useEntitlementsStore } from "../store/entitlementsStore";
import { useStreamCeilingStore } from "../store/streamCeilingStore";

export function useStreamQualitySync() {
  const tier = useEntitlementsStore((s) => s.tier);
  const wanted = useVoiceSettingsStore((s) => s.streamQuality);
  const ceiling = useStreamCeilingStore((s) => s.ceiling);
  React.useEffect(() => {
    const quality = bestAllowedQuality(wanted, tier, ceiling);
    void getLiveKitService().setScreenShareQuality({
      width: quality.width,
      height: quality.height,
      frameRate: quality.frameRate,
      maxBitrate: quality.maxBitrate,
      adaptive: wanted === "auto",
    });
  }, [tier, wanted, ceiling]);
}
