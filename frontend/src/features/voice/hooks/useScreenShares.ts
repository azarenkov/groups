import { useCallback, useEffect, useState } from "react";

import { getLiveKitService } from "../lib/livekit";
import type { ScreenShareState } from "../lib/livekit";

export interface ScreenShares extends ScreenShareState {
  watch: (identity: string | null) => void;
}

export function useScreenShares(): ScreenShares {
  const [state, setState] = useState<ScreenShareState>({
    shares: [],
    watching: null,
  });

  useEffect(() => {
    const service = getLiveKitService();
    const release = service.acquireScreenView();
    const unsubscribe = service.subscribeScreenShares(setState);
    return () => {
      unsubscribe();
      release();
    };
  }, []);

  const watch = useCallback((identity: string | null) => {
    getLiveKitService().watchScreen(identity);
  }, []);

  return { ...state, watch };
}
