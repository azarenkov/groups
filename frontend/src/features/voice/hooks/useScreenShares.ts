import { useCallback, useEffect, useState } from "react";

import { getLiveKitService } from "../lib/livekit";
import type { ScreenShareState } from "../lib/livekit";

export interface ScreenShares extends ScreenShareState {
  watch: (identity: string | null) => void;
  setWatched: (identity: string, on: boolean) => void;
}

export function useScreenShares(visible: boolean = true): ScreenShares {
  const [state, setState] = useState<ScreenShareState>({
    shares: [],
    watching: null,
    watched: [],
  });

  useEffect(() => getLiveKitService().subscribeScreenShares(setState), []);

  useEffect(() => {
    if (!visible) return undefined;
    return getLiveKitService().acquireScreenView();
  }, [visible]);

  const watch = useCallback((identity: string | null) => {
    getLiveKitService().watchScreen(identity);
  }, []);

  const setWatched = useCallback((identity: string, on: boolean) => {
    getLiveKitService().setScreenWatched(identity, on);
  }, []);

  return { ...state, watch, setWatched };
}
