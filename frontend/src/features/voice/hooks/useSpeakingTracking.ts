import { useEffect } from "react";

import { useAuthStore } from "../../auth/store/authStore";
import { startSpeakingTracking } from "../store/speakingStore";

export function useSpeakingTracking(): void {
  const selfId = useAuthStore((s) => s.user?.id ?? null);

  useEffect(() => startSpeakingTracking(selfId), [selfId]);
}
