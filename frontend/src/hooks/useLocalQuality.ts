import { useEffect, useState } from "react";
import { ConnectionQuality, getLiveKitService } from "../services/livekit";

export function useLocalQuality(): ConnectionQuality {
  const [quality, setQuality] = useState<ConnectionQuality>(
    ConnectionQuality.Unknown,
  );

  useEffect(() => {
    const service = getLiveKitService();
    setQuality(service.getLocalQuality());
    return service.subscribeLocalQuality(setQuality);
  }, []);

  return quality;
}
