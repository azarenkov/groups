import { useEffect, useState } from "react";
import { getLiveKitService, ParticipantInfo } from "../services/livekit";

export function useParticipants(): ParticipantInfo[] {
  const [participants, setParticipants] = useState<ParticipantInfo[]>([]);

  useEffect(() => {
    const service = getLiveKitService();
    setParticipants(service.getParticipants());
    return service.subscribeParticipants(setParticipants);
  }, []);

  return participants;
}
