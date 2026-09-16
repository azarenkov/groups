import { useEffect, useState } from "react";
import { getLiveKitService } from "../lib/livekit";

export function useLocalSpeaking(): boolean {
  const [speaking, setSpeaking] = useState(false);

  useEffect(() => {
    const service = getLiveKitService();
    setSpeaking(service.getLocalSpeaking());
    return service.subscribeLocalSpeaking(setSpeaking);
  }, []);

  return speaking;
}
