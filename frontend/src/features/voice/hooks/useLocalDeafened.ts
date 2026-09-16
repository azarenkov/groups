import { useEffect, useState } from "react";
import { getLiveKitService } from "../lib/livekit";

export function useLocalDeafened(): boolean {
  const [deafened, setDeafened] = useState(false);

  useEffect(() => {
    const service = getLiveKitService();
    setDeafened(service.isDeafened());
    return service.subscribeLocalDeafened(setDeafened);
  }, []);

  return deafened;
}
