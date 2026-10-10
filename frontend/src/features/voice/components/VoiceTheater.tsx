import React, { useEffect, useRef, useState } from "react";
import i18nTheater from "../../../i18n";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { VideoTileCell } from "./VideoStage";
import {
  getLiveKitService,
  REMOTE_VOLUME_MAX,
  REMOTE_VOLUME_PLAIN_MAX,
  VideoTile,
} from "../lib/livekit";
import type { ScreenShareInfo } from "../lib/livekit";
import { UserAvatar } from "../../../shared/ui/Avatar";
import {
  openContextMenu,
  openContextMenuAt,
} from "../../../shared/ui/contextMenu/contextMenuStore";
import type { MenuEntry } from "../../../shared/ui/contextMenu/contextMenuStore";
import { IDLE_AUTO_FOCUS, autoFocusWait, nextAutoFocus } from "../lib/stageFocus";

const TileVolumeMenu: React.FC<{
  id: string;
  className?: string;
  moderate?: { muted: boolean; onToggle: () => void };
  deafen?: { deafened: boolean; onToggle: () => void };
}> = ({ id, className, moderate, deafen }) => {
  const [open, setOpen] = useState(false);
  const [volMax, setVolMax] = useState(() =>
    getLiveKitService().supportsRemoteBoost(id)
      ? REMOTE_VOLUME_MAX
      : REMOTE_VOLUME_PLAIN_MAX,
  );
  const [vol, setVol] = useState(() =>
    Math.min(volMax, Math.round(getLiveKitService().getRemoteVolume(id) * 100)),
  );
  const btnRef = useRef<HTMLButtonElement>(null);
  const popRef = useRef<HTMLDivElement>(null);

  useEffect(
    () =>
      getLiveKitService().subscribeRemoteVolume((who, volume) => {
        if (who === id) setVol(Math.round(volume * 100));
      }),
    [id],
  );
  const [pos, setPos] = useState<{ top: number; left: number } | null>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        popRef.current &&
        !popRef.current.contains(target) &&
        btnRef.current &&
        !btnRef.current.contains(target)
      ) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const toggle = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (open) {
      setOpen(false);
      return;
    }
    const service = getLiveKitService();
    const max = service.supportsRemoteBoost(id)
      ? REMOTE_VOLUME_MAX
      : REMOTE_VOLUME_PLAIN_MAX;
    const stored = Math.round(service.getRemoteVolume(id) * 100);
    const allowed = Math.min(stored, max);
    if (allowed !== stored) service.setRemoteVolume(id, allowed / 100);
    setVolMax(max);
    setVol(allowed);
    const r = btnRef.current?.getBoundingClientRect();
    if (r) {
      const menuW = 48;
      const menuH = 178 + (moderate ? 36 : 0) + (deafen ? 36 : 0);
      let left = r.left - menuW - 8;
      if (left < 8) left = Math.min(r.right + 8, window.innerWidth - menuW - 8);
      let top = r.top;
      if (top + menuH > window.innerHeight - 8) {
        top = window.innerHeight - menuH - 8;
      }
      setPos({ top, left });
    }
    setOpen(true);
  };

  const change = (v: number) => {
    setVol(v);
    getLiveKitService().setRemoteVolume(id, v / 100);
  };

  return (
    <>
      <button
        ref={btnRef}
        type="button"
        className={
          "cs-menu-btn" +
          (className ? ` ${className}` : "") +
          (open ? " on" : "") +
          (vol !== 100 ? " adjusted" : "")
        }
        onClick={toggle}
        title={`${i18nTheater.t("voice:volume")} ${vol}%`}
        aria-label={`${i18nTheater.t("voice:volume")} ${vol}%`}
        aria-haspopup="menu"
        aria-expanded={open}
      >
        <Icon
          name={vol === 0 ? "speaker-off" : vol < 40 ? "speaker-lo" : "speaker-hi"}
          size={14}
        />
        {vol !== 100 && <span className="cs-menu-btn-pct">{vol}%</span>}
      </button>
      {open &&
        pos &&
        createPortal(
          <div
            ref={popRef}
            className="cs-menu-pop"
            style={{ top: pos.top, left: pos.left }}
            onMouseDown={(e) => e.stopPropagation()}
          >
            <div className="cs-menu-vol">
              <span className="cs-menu-pct">{vol}%</span>
              <div className="cs-menu-track">
                <input
                  type="range"
                  min={0}
                  max={volMax}
                  step={1}
                  value={vol}
                  aria-label={i18nTheater.t("voice:volume")}
                  onChange={(e) => change(Number(e.target.value))}
                  style={
                    {
                      ["--pct" as unknown as string]: `${(vol / volMax) * 100}%`,
                    } as React.CSSProperties
                  }
                />
              </div>
              <button
                type="button"
                className="cs-menu-mute"
                title={i18nTheater.t("voice:volume")}
                aria-label={i18nTheater.t("voice:volume")}
                onClick={() => change(vol === 0 ? 100 : 0)}
              >
                <Icon
                  name={
                    vol === 0 ? "speaker-off" : vol < 40 ? "speaker-lo" : "speaker-hi"
                  }
                  size={17}
                />
              </button>
            </div>
            {moderate && (
              <button
                type="button"
                className="cs-menu-item"
                title={moderate.muted ? i18nTheater.t("voice:unmuteMember") : i18nTheater.t("voice:muteMember")}
                aria-label={moderate.muted ? i18nTheater.t("voice:unmuteMember") : i18nTheater.t("voice:muteMember")}
                onClick={() => {
                  moderate.onToggle();
                  setOpen(false);
                }}
              >
                <Icon name={moderate.muted ? "mic" : "mic-off"} size={16} />
              </button>
            )}
            {deafen && (
              <button
                type="button"
                className="cs-menu-item"
                title={deafen.deafened ? i18nTheater.t("voice:undeafenMember") : i18nTheater.t("voice:deafenMember")}
                aria-label={deafen.deafened ? i18nTheater.t("voice:undeafenMember") : i18nTheater.t("voice:deafenMember")}
                onClick={() => {
                  deafen.onToggle();
                  setOpen(false);
                }}
              >
                <Icon name={deafen.deafened ? "headphones" : "headphones-off"} size={16} />
              </button>
            )}
          </div>,
          document.body,
        )}
    </>
  );
};

export interface TheaterEntry {
  id: string;
  name: string;
  isSelf: boolean;
  color: string;
  initials: string;
  speaking: boolean;
  muted: boolean;
  deafened: boolean;
  moderatorDeafened: boolean;
  camera: VideoTile | null;
}

interface VoiceTheaterProps {
  roster: TheaterEntry[];
  screenTile: VideoTile | null;
  screenTiles?: VideoTile[];
  watched?: string[];
  onSetWatched?: (identity: string, on: boolean) => void;
  liveName?: string | null;
  onMinimize?: () => void;
  minimizeTitle?: string;
  viewersFor?: (id: string) => number | undefined;
  onWatchChange?: (id: string, watching: boolean) => void;
  shares?: ScreenShareInfo[];
  watching?: string | null;
  onWatch?: (identity: string | null) => void;
  onModerateMute?: (id: string, currentlyMuted: boolean) => void;
  onModerateDeafen?: (id: string, currentlyDeafened: boolean) => void;
  moderationExcluded?: string[];
  deafenExcluded?: string[];
}

const STRIP_MAX = 6;

const gridColumns = (count: number): number => {
  if (count <= 2) return Math.max(1, count);
  if (count <= 4) return 2;
  if (count <= 9) return 3;
  return 4;
};

const VoiceTheater: React.FC<VoiceTheaterProps> = ({
  roster,
  screenTile,
  screenTiles,
  watched,
  onSetWatched,
  liveName,
  onMinimize,
  minimizeTitle,
  viewersFor,
  onWatchChange,
  shares,
  watching,
  onWatch,
  onModerateMute,
  onModerateDeafen,
  moderationExcluded,
  deafenExcluded,
}) => {
  const { t } = useTranslation(["call", "voice"]);
  const available = shares ?? [];
  const sharingIds = new Set(available.map((share) => share.identity));
  const offered = watching === null || watching === undefined ? available : [];
  const [pinnedId, setPinnedId] = useState<string | null>(null);
  const [pinnedScreen, setPinnedScreen] = useState<string | null>(null);
  const watchedTiles = screenTiles ?? (screenTile ? [screenTile] : []);
  const watchedIds = watched ?? (watching ? [watching] : []);
  const isWatched = (id: string) => watchedIds.includes(id);
  const [autoFocus, setAutoFocus] = useState(IDLE_AUTO_FOCUS);
  const [focusTick, setFocusTick] = useState(0);
  const candidates = roster.map((r) => ({
    id: r.id,
    speaking: r.speaking,
    hasVideo: !!r.camera,
    isSelf: r.isSelf,
  }));
  const candidatesRef = useRef(candidates);
  candidatesRef.current = candidates;
  const candidatesKey = candidates
    .map((c) => `${c.id}:${c.speaking ? 1 : 0}${c.hasVideo ? 1 : 0}`)
    .join("|");
  useEffect(() => {
    setAutoFocus((prev) => nextAutoFocus(prev, candidatesRef.current, performance.now()));
  }, [candidatesKey, focusTick]);
  useEffect(() => {
    const wait = autoFocusWait(autoFocus, performance.now());
    if (wait === null) return;
    const timer = setTimeout(() => setFocusTick((n) => n + 1), wait + 10);
    return () => clearTimeout(timer);
  }, [autoFocus]);
  const pinnedCamera = pinnedId
    ? (roster.find((r) => r.id === pinnedId)?.camera ?? null)
    : null;
  const pinnedScreenTile = pinnedScreen
    ? (watchedTiles.find((tile) => tile.identity === pinnedScreen) ?? null)
    : null;
  useEffect(() => {
    if (pinnedId && !pinnedCamera) setPinnedId(null);
  }, [pinnedId, pinnedCamera]);
  useEffect(() => {
    if (pinnedScreen && !pinnedScreenTile) setPinnedScreen(null);
  }, [pinnedScreen, pinnedScreenTile]);
  const pinnedTile = pinnedScreenTile ?? pinnedCamera;
  const togglePin = (id: string) => {
    setPinnedScreen(null);
    setPinnedId((prev) => (prev === id ? null : id));
  };
  const toggleScreenPin = (id: string) => {
    setPinnedId(null);
    setPinnedScreen((prev) => (prev === id ? null : id));
  };
  const unpin = () => {
    setPinnedId(null);
    setPinnedScreen(null);
  };
  const cameraEntries = roster.filter((r) => r.camera);
  const gridKind: "screens" | "cameras" | null = pinnedTile
    ? null
    : available.length >= 2
      ? "screens"
      : available.length === 0 && cameraEntries.length >= 2
        ? "cameras"
        : null;
  const gridCount = gridKind === "screens" ? available.length : cameraEntries.length;
  const watchShare = (id: string) => {
    if (gridKind === "screens" && onSetWatched) onSetWatched(id, true);
    else onWatch?.(id);
  };
  const mainVideo =
    pinnedTile ??
    screenTile ??
    roster.find((r) => r.id === autoFocus.current)?.camera ??
    roster.find((r) => !r.isSelf && r.camera)?.camera ??
    roster.find((r) => r.camera)?.camera ??
    null;
  const videoSources = roster.filter((r) => r.camera).length + (screenTile ? 1 : 0);
  const showFocus = !gridKind && !!mainVideo && (!!pinnedTile || videoSources > 1);
  const pinnedOwner = pinnedTile ? roster.find((r) => r.id === pinnedTile.identity) : undefined;
  const pinnedName = pinnedTile
    ? pinnedScreenTile
      ? pinnedScreenTile.isLocal
        ? t("yourScreen")
        : (pinnedOwner?.name ?? "")
      : pinnedOwner?.isSelf
        ? t("you")
        : (pinnedOwner?.name ?? "")
    : "";
  const stripSource = gridKind === "cameras" ? roster.filter((r) => !r.camera) : roster;
  const strip = stripSource.slice(0, STRIP_MAX);
  const overflow = stripSource.slice(STRIP_MAX);
  const overflowMenu: MenuEntry[] = overflow
    .filter((r) => r.camera)
    .map((r) => ({
      kind: "item" as const,
      id: `pin-${r.id}`,
      label: `${pinnedId === r.id && pinnedTile ? t("unpinFromStage") : t("pinToStage")}: ${r.isSelf ? t("you") : r.name}`,
      icon: "pin" as const,
      onSelect: () => togglePin(r.id),
    }));
  const stageOwner = mainVideo && mainVideo.source === "camera" ? mainVideo.identity : null;
  const tileMenu = (e: React.MouseEvent, id: string) =>
    openContextMenu(e, [
      {
        kind: "item",
        id: "pin",
        label: pinnedId === id && pinnedTile ? t("unpinFromStage") : t("pinToStage"),
        icon: "pin",
        onSelect: () => togglePin(id),
      },
    ]);
  const screenMenu = (e: React.MouseEvent, id: string) =>
    openContextMenu(e, [
      {
        kind: "item",
        id: "pin",
        label: pinnedScreen === id && pinnedTile ? t("unpinFromStage") : t("pinToStage"),
        icon: "pin",
        onSelect: () => toggleScreenPin(id),
      },
      ...(onSetWatched
        ? [
            {
              kind: "item" as const,
              id: "stop",
              label: t("stopWatching"),
              icon: "x" as const,
              onSelect: () => onSetWatched(id, false),
            },
          ]
        : []),
    ]);
  const stageScreenOwner =
    mainVideo && mainVideo.source === "screen" ? mainVideo.identity : null;
  const mainName = mainVideo
    ? (roster.find((r) => r.id === mainVideo.identity)?.name ?? "")
    : "";

  return (
    <div className="call-theater">
      <div className="call-stage">
        {onMinimize && (
          <button
            type="button"
            className="call-min"
            title={minimizeTitle ?? t("minimize")}
            onClick={onMinimize}
          >
            <Icon name="chevron" size={16} />
          </button>
        )}

        {showFocus && (
          <button
            type="button"
            className={"call-focus" + (pinnedTile ? " pinned" : "")}
            title={pinnedTile ? t("unpinFromStage") : t("stageAutoHint")}
            onClick={pinnedTile ? unpin : undefined}
            disabled={!pinnedTile}
          >
            <Icon name={pinnedTile ? "pin" : "volume"} size={13} />
            <span>{pinnedTile ? pinnedName : t("stageAuto")}</span>
          </button>
        )}

        {!gridKind && offered.length > 0 && onWatch && (
          <div className="call-offer">
            <div className="call-offer-title">{t("sharingNow")}</div>
            <div className="call-offer-list">
              {offered.map((share) => (
                <button
                  key={share.identity}
                  type="button"
                  className="call-offer-btn"
                  onClick={() => onWatch(share.identity)}
                >
                  <span className="call-live-dot" />
                  <span className="call-offer-name">
                    {share.isLocal ? t("yourScreen") : share.name}
                  </span>
                  <span className="call-offer-cta">{t("watchStream")}</span>
                </button>
              ))}
            </div>
          </div>
        )}

        {gridKind ? (
          <div
            className={"call-grid" + (onMinimize ? " with-min" : "")}
            style={
              {
                "--cg-cols": gridColumns(gridCount),
                "--cg-rows": Math.ceil(gridCount / gridColumns(gridCount)),
              } as React.CSSProperties
            }
          >
            {gridKind === "screens"
              ? available.map((share) => {
                  const tile = watchedTiles.find((v) => v.identity === share.identity) ?? null;
                  const owner = roster.find((r) => r.id === share.identity);
                  const name = share.isLocal ? t("yourScreen") : share.name;
                  if (!tile) {
                    return (
                      <div key={share.identity} className="cg-cell idle">
                        <span className="cs-tile-live cg-live">
                          <span className="call-live-dot" />
                          LIVE
                        </span>
                        <UserAvatar
                          userId={share.identity}
                          name={share.name}
                          className="cg-ava"
                          style={{ background: owner?.color }}
                        />
                        <div className="cg-name">{name}</div>
                        <button
                          type="button"
                          className="cg-watch"
                          onClick={() => watchShare(share.identity)}
                        >
                          <Icon name="eye" size={14} />
                          {t("watchStream")}
                        </button>
                      </div>
                    );
                  }
                  return (
                    <div
                      key={share.identity}
                      className={"cg-cell" + (owner?.speaking ? " speaking" : "")}
                      onClick={() => toggleScreenPin(share.identity)}
                      onContextMenu={(e) => screenMenu(e, share.identity)}
                      title={t("pinToStage")}
                    >
                      <VideoTileCell
                        tile={tile}
                        label={owner?.name ?? share.name}
                        viewers={viewersFor ? viewersFor(share.identity) : undefined}
                        onWatchChange={onWatchChange}
                      />
                      {onSetWatched && (
                        <button
                          type="button"
                          className="cg-close"
                          title={t("stopWatching")}
                          aria-label={t("stopWatching")}
                          onClick={(e) => {
                            e.stopPropagation();
                            onSetWatched(share.identity, false);
                          }}
                        >
                          <Icon name="x" size={13} />
                        </button>
                      )}
                    </div>
                  );
                })
              : cameraEntries.map((r) =>
                  r.camera ? (
                    <div
                      key={r.id}
                      className={"cg-cell cam" + (r.speaking ? " speaking" : "")}
                      onClick={() => togglePin(r.id)}
                      onContextMenu={(e) => tileMenu(e, r.id)}
                      title={t("pinToStage")}
                    >
                      <VideoTileCell
                        tile={r.camera}
                        label={r.isSelf ? t("you") : r.name}
                      />
                      {!r.isSelf && (
                        <div className="cg-menu" onClick={(e) => e.stopPropagation()}>
                          <TileVolumeMenu
                            id={r.id}
                            moderate={
                              onModerateMute && !moderationExcluded?.includes(r.id)
                                ? {
                                    muted: r.muted,
                                    onToggle: () => onModerateMute(r.id, r.muted),
                                  }
                                : undefined
                            }
                            deafen={
                              onModerateDeafen &&
                              !moderationExcluded?.includes(r.id) &&
                              !deafenExcluded?.includes(r.id)
                                ? {
                                    deafened: r.moderatorDeafened,
                                    onToggle: () =>
                                      onModerateDeafen(r.id, r.moderatorDeafened),
                                  }
                                : undefined
                            }
                          />
                        </div>
                      )}
                      {(r.muted || r.deafened) && (
                        <span className="cg-muted">
                          {r.muted && <Icon name="mic-off" size={12} />}
                          {r.deafened && <Icon name="headphones-off" size={12} />}
                        </span>
                      )}
                    </div>
                  ) : null,
                )}
          </div>
        ) : mainVideo ? (
          <div
            className="call-screen"
            onContextMenu={
              stageOwner
                ? (e) => tileMenu(e, stageOwner)
                : stageScreenOwner && available.length > 1
                  ? (e) => screenMenu(e, stageScreenOwner)
                  : undefined
            }
          >
            <VideoTileCell
              key={`${mainVideo.identity}:${mainVideo.source}`}
              tile={mainVideo}
              label={mainName}
              viewers={
                mainVideo.source === "screen" && viewersFor
                  ? viewersFor(mainVideo.identity)
                  : undefined
              }
              onWatchChange={onWatchChange}
            />
            {stageScreenOwner && (
              <div className="call-live">
                <span className="call-live-badge">
                  <span className="call-live-dot" />
                  LIVE
                </span>
                <span className="call-live-name">
                  {t("sharingScreen", { name: mainName || liveName || "" })}
                </span>
                {onWatch && watching && available.length <= 1 && (
                  <button
                    type="button"
                    className="call-live-chip stop"
                    onClick={() => onWatch(null)}
                  >
                    {t("stopWatching")}
                  </button>
                )}
              </div>
            )}
          </div>
        ) : (
          <div className="call-screen empty">
            <div className="call-empty-orbs">
              {roster.map((r) => (
                <div
                  key={r.id}
                  className={"call-empty-cell" + (r.speaking ? " speaking" : "")}
                >
                  <UserAvatar
                    userId={r.id}
                    name={r.name}
                    className={"call-empty-orb" + (r.speaking ? " speaking" : "")}
                    style={{ "--orb-color": r.color } as React.CSSProperties}
                  >
                    {r.muted && (
                      <span className="call-empty-mute">
                        <Icon name="mic-off" size={16} />
                      </span>
                    )}
                    {r.deafened && (
                      <span className="call-empty-mute deaf">
                        <Icon name="headphones-off" size={16} />
                      </span>
                    )}
                    {!r.isSelf && (
                      <TileVolumeMenu
                        id={r.id}
                        className="call-orb-menu"
                        moderate={
                          onModerateMute && !moderationExcluded?.includes(r.id)
                            ? {
                                muted: r.muted,
                                onToggle: () => onModerateMute(r.id, r.muted),
                              }
                            : undefined
                        }
                        deafen={
                          onModerateDeafen &&
                          !moderationExcluded?.includes(r.id) &&
                          !deafenExcluded?.includes(r.id)
                            ? {
                                deafened: r.moderatorDeafened,
                                onToggle: () =>
                                  onModerateDeafen(r.id, r.moderatorDeafened),
                              }
                            : undefined
                        }
                      />
                    )}
                  </UserAvatar>
                  <div className="call-empty-name">
                    {r.isSelf ? t("you") : r.name}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

      </div>

      {(mainVideo || gridKind) && stripSource.length > 0 && (
      <div className="call-strip">
        {strip.map((r) => (
            <div
              key={r.id}
              className={
                "cs-tile" +
                (r.speaking ? " speaking" : "") +
                (sharingIds.has(r.id) ? " sharing" : "") +
                (isWatched(r.id) ? " watched" : "") +
                (pinnedId === r.id && pinnedTile ? " pinned" : "") +
                (r.camera ? " pinnable" : "")
              }
              onClick={
                onWatch && sharingIds.has(r.id) && !isWatched(r.id)
                  ? () => watchShare(r.id)
                  : r.camera
                    ? () => togglePin(r.id)
                    : undefined
              }
              onContextMenu={r.camera ? (e) => tileMenu(e, r.id) : undefined}
              title={
                onWatch && sharingIds.has(r.id) && !isWatched(r.id)
                  ? t("watchStream")
                  : r.camera
                    ? pinnedId === r.id
                      ? t("unpinFromStage")
                      : t("pinToStage")
                    : undefined
              }
            >
              {pinnedId === r.id && pinnedTile && (
                <span className="cs-tile-pin">
                  <Icon name="pin" size={11} />
                </span>
              )}
              {sharingIds.has(r.id) && (
                <span className="cs-tile-live">
                  <span className="call-live-dot" />
                  LIVE
                </span>
              )}
              {r.camera ? (
                <VideoTileCell tile={r.camera} label="" />
              ) : (
                <UserAvatar
                  userId={r.id}
                  name={r.name}
                  className="cs-tile-ava"
                  style={{ background: r.color }}
                />
              )}
              <div className={"cs-tile-name" + (r.muted ? " muted" : "")}>
                <Icon name={r.muted ? "mic-off" : "mic"} size={12} />
                {r.deafened && <Icon name="headphones-off" size={12} />}
                <span>{r.isSelf ? t("you") : r.name}</span>
              </div>
              {!r.isSelf && (
                <TileVolumeMenu
                  id={r.id}
                  className="cs-tile-menu"
                  moderate={
                    onModerateMute && !moderationExcluded?.includes(r.id)
                      ? {
                          muted: r.muted,
                          onToggle: () => onModerateMute(r.id, r.muted),
                        }
                      : undefined
                  }
                  deafen={
                    onModerateDeafen &&
                    !moderationExcluded?.includes(r.id) &&
                    !deafenExcluded?.includes(r.id)
                      ? {
                          deafened: r.moderatorDeafened,
                          onToggle: () =>
                            onModerateDeafen(r.id, r.moderatorDeafened),
                        }
                      : undefined
                  }
                />
              )}
            </div>
          ))}
        {overflow.length > 0 && (
          <div
            className={"cs-more" + (overflowMenu.length > 0 ? " clickable" : "")}
            role={overflowMenu.length > 0 ? "button" : undefined}
            tabIndex={overflowMenu.length > 0 ? 0 : undefined}
            onClick={overflowMenu.length > 0 ? (e) => openContextMenu(e, overflowMenu) : undefined}
            onContextMenu={overflowMenu.length > 0 ? (e) => openContextMenu(e, overflowMenu) : undefined}
            onKeyDown={
              overflowMenu.length > 0
                ? (e) => {
                    if (e.key !== "Enter" && e.key !== " ") return;
                    e.preventDefault();
                    openContextMenuAt(e.currentTarget, overflowMenu);
                  }
                : undefined
            }
          >
            <div className="cs-more-avas">
              {overflow.slice(0, 2).map((r) => (
                <span key={r.id} style={{ background: r.color }} />
              ))}
            </div>
            <span className="cs-more-count">+{overflow.length}</span>
          </div>
        )}
      </div>
      )}
    </div>
  );
};

export default VoiceTheater;
