import { useEffect, useState } from "react";
import { getLiveKitService, VideoTile } from "../services/livekit";

export function useVideoTiles(): VideoTile[] {
  const [tiles, setTiles] = useState<VideoTile[]>([]);

  useEffect(() => {
    const service = getLiveKitService();
    const update = () => setTiles(service.getVideoTiles());
    update();
    return service.subscribeParticipants(update);
  }, []);

  return tiles;
}
