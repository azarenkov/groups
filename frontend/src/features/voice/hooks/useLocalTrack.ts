import { useEffect, useState } from "react";
import { getLiveKitService } from "../lib/livekit";

export function useLocalTrack(): MediaStreamTrack | null {
  const [track, setTrack] = useState<MediaStreamTrack | null>(null);

  useEffect(() => {
    const service = getLiveKitService();
    setTrack(service.getLocalAudioMediaStreamTrack());
    return service.subscribeLocalTrack(setTrack);
  }, []);

  return track;
}
