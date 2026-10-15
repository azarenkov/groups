import React from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { APP_ROOT, appPath, parseAppPath, sameLocation, type AppLocation } from "../lib/appLocation";
import { lastLocationPath, rememberLocation } from "../lib/lastLocation";

const HOME: AppLocation = { kind: "home" };

export function useLocationSync(params: {
  viewerId: string | undefined;
  current: AppLocation | null;
  resolving: boolean;
  open: (location: AppLocation) => void;
}): void {
  const { viewerId, current, resolving, open } = params;
  const location = useLocation();
  const navigate = useNavigate();
  const openRef = React.useRef(open);
  openRef.current = open;
  const currentKey = current ? appPath(current) : null;
  const seen = React.useRef<{ entry: string | null; current: string | null }>({ entry: null, current: null });
  const urlLocation = React.useMemo(() => parseAppPath(location.pathname, viewerId), [location.pathname, viewerId]);

  React.useEffect(() => {
    const path = location.pathname.replace(/\/+$/, "");
    const entryChanged = seen.current.entry !== location.key;
    const currentChanged = seen.current.current !== currentKey;
    seen.current = { entry: location.key, current: resolving && currentChanged && !entryChanged ? seen.current.current : currentKey };
    if (!viewerId) return;
    if (path === APP_ROOT) {
      navigate({ pathname: lastLocationPath(viewerId) ?? appPath(HOME), search: location.search }, { replace: true });
      return;
    }
    if (entryChanged) {
      if (!urlLocation) {
        navigate({ pathname: appPath(HOME), search: location.search }, { replace: true });
        return;
      }
      const jump = "messageId" in urlLocation && !!urlLocation.messageId;
      if (urlLocation.kind !== "settings" && (jump || !sameLocation(urlLocation, current))) openRef.current(urlLocation);
      return;
    }
    if (currentChanged && current && !resolving && urlLocation?.kind !== "settings" && !sameLocation(urlLocation, current)) {
      const settlesCommunity =
        urlLocation?.kind === "community" && current.kind === "channel" && current.communityId === urlLocation.communityId;
      navigate(appPath(current), { replace: settlesCommunity });
    }
  }, [location.key, location.pathname, location.search, urlLocation, current, currentKey, resolving, viewerId, navigate]);

  React.useEffect(() => {
    if (viewerId && current && sameLocation(urlLocation, current)) rememberLocation(viewerId, appPath(current));
  }, [viewerId, current, currentKey, urlLocation]);

}
