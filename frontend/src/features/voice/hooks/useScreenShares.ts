import { useCallback, useEffect, useState } from "react";

import { getLiveKitService } from "../lib/livekit";
import type { ScreenShareState } from "../lib/livekit";

export interface ScreenShares extends ScreenShareState {
  watch: (identity: string | null) => void;
}

export function useScreenShares(visible: boolean = true): ScreenShares {
  const [state, setState] = useState<ScreenShareState>({
    shares: [],
    watching: null,
  });

  useEffect(() => getLiveKitService().subscribeScreenShares(setState), []);

  useEffect(() => {
    if (!visible) return undefined;
    return getLiveKitService().acquireScreenView();
  }, [visible]);

  const watch = useCallback((identity: string | null) => {
    getLiveKitService().watchScreen(identity);
  }, []);

  return { ...state, watch };
}
