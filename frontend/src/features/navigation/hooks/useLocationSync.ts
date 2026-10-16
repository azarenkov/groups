import React from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { APP_ROOT, appPath, legacyMessagePath, parseAppPath, sameLocation, type AppLocation } from "../lib/appLocation";
import { lastLocationPath, rememberLocation } from "../lib/lastLocation";

const HOME: AppLocation = { kind: "home" };

export function useLocationSync(params: {
  viewerId: string | undefined;
  current: AppLocation | null;
  selectionSeq: number;
  resolving: boolean;
  open: (location: AppLocation) => void;
  leaveMessage: () => void;
}): void {
  const { viewerId, current, selectionSeq, resolving, open, leaveMessage } = params;
  const location = useLocation();
  const navigate = useNavigate();
  const openRef = React.useRef(open);
  openRef.current = open;
  const leaveMessageRef = React.useRef(leaveMessage);
  leaveMessageRef.current = leaveMessage;
  const currentKey = current ? appPath(current) : null;
  const seen = React.useRef<{ entry: string | null; current: string | null; selection: number; message: boolean }>({
    entry: null,
    current: null,
    selection: selectionSeq,
    message: false,
  });
  const urlLocation = React.useMemo(() => parseAppPath(location.pathname, viewerId), [location.pathname, viewerId]);

  React.useEffect(() => {
    const path = location.pathname.replace(/\/+$/, "");
    const entryChanged = seen.current.entry !== location.key;
    const currentChanged = seen.current.current !== currentKey;
    const jump = !!urlLocation && "messageId" in urlLocation && !!urlLocation.messageId;
    const leftMessage = entryChanged && seen.current.message && !jump;
    const reselected = seen.current.selection !== selectionSeq;
    const deferred = resolving && !entryChanged;
    seen.current = {
      entry: location.key,
      current: deferred && currentChanged ? seen.current.current : currentKey,
      selection: deferred ? seen.current.selection : selectionSeq,
      message: entryChanged ? jump : seen.current.message,
    };
    if (!viewerId) return;
    const legacy = legacyMessagePath(new URLSearchParams(location.search));
    if (legacy || path === APP_ROOT) {
      navigate(legacy ?? lastLocationPath(viewerId) ?? appPath(HOME), { replace: true });
      return;
    }
    if (entryChanged) {
      if (!urlLocation) {
        navigate({ pathname: appPath(HOME), search: location.search }, { replace: true });
        return;
      }
      if (urlLocation.kind === "settings") return;
      if (jump || !sameLocation(urlLocation, current)) openRef.current(urlLocation);
      else if (leftMessage) leaveMessageRef.current();
      return;
    }
    const stale = !sameLocation(urlLocation, current) || (reselected && jump);
    if ((currentChanged || reselected) && current && !resolving && urlLocation?.kind !== "settings" && stale) {
      const settlesCommunity =
        urlLocation?.kind === "community" && current.kind === "channel" && current.communityId === urlLocation.communityId;
      navigate(appPath(current), { replace: settlesCommunity });
    }
  }, [location.key, location.pathname, location.search, urlLocation, current, currentKey, selectionSeq, resolving, viewerId, navigate]);

  React.useEffect(() => {
    if (viewerId && current && sameLocation(urlLocation, current)) rememberLocation(viewerId, appPath(current));
  }, [viewerId, current, currentKey, urlLocation]);

}
