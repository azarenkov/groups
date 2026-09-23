import React from "react";

import { getLiveKitService } from "../../voice/lib/livekit";
import { useVoiceSettingsStore } from "../../voice/store/voiceSettingsStore";
import { bestAllowedQuality } from "../lib/plans";
import { useEntitlementsStore } from "../store/entitlementsStore";

export function useStreamQualitySync() {
  const tier = useEntitlementsStore((s) => s.tier);
  const wanted = useVoiceSettingsStore((s) => s.streamQuality);
  React.useEffect(() => {
    const quality = bestAllowedQuality(wanted, tier);
    void getLiveKitService().setScreenShareQuality({
      width: quality.width,
      height: quality.height,
      frameRate: quality.frameRate,
      maxBitrate: quality.maxBitrate,
      adaptive: wanted === "auto",
    });
  }, [tier, wanted]);
}
