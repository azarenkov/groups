import React from "react";
import { useLocation, useNavigate, useNavigationType } from "react-router-dom";

import { useHotkey } from "../../settings/lib/hotkeys";

export function useBrowserHistory() {
  const navigate = useNavigate();
  const location = useLocation();
  const navigationType = useNavigationType();
  const entries = React.useRef<string[]>([location.key]);
  const index = React.useRef(0);
  const [navState, setNavState] = React.useState({ back: false, forward: false });

  React.useEffect(() => {
    const key = location.key;
    if (navigationType === "PUSH") {
      entries.current = [...entries.current.slice(0, index.current + 1), key];
      index.current = entries.current.length - 1;
    } else if (navigationType === "REPLACE") {
      entries.current[index.current] = key;
    } else {
      const found = entries.current.indexOf(key);
      if (found >= 0) index.current = found;
    }
    setNavState({ back: index.current > 0, forward: index.current < entries.current.length - 1 });
  }, [location.key, navigationType]);

  const goHistory = React.useCallback(
    (delta: -1 | 1) => {
      const next = index.current + delta;
      if (next >= 0 && next < entries.current.length) navigate(delta);
    },
    [navigate],
  );
  useHotkey("historyBack", () => goHistory(-1));
  useHotkey("historyForward", () => goHistory(1));

  return { navState, goHistory };
}
