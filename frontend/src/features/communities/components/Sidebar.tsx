import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import type { Community } from "../../../shared/types/community";
import type { Channel, ChannelKind } from "../../../shared/types/channel";
import type { DirectConversation } from "../../dm/types";
import ChannelRow from "./sidebar/ChannelRow";
import DmList from "./sidebar/DmList";
import CommunityList from "./sidebar/CommunityList";
import CommunityPage from "./sidebar/CommunityPage";
import SidebarHeader from "./sidebar/SidebarHeader";
import { pinnedFirst, useConversationPinsStore } from "../store/conversationPinsStore";
import { hiddenDmsReady, useHiddenDmStore } from "../../dm/store/hiddenDmStore";
import { useSidebarPermissions } from "./sidebar/permissions";
import type { SidebarVoiceMember } from "./sidebar/types";
import {
  SIDEBAR_COLLAPSED_WIDTH,
  SIDEBAR_MAX_WIDTH,
  SIDEBAR_MIN_WIDTH,
  useSidebarLayoutStore,
} from "../store/sidebarLayoutStore";
import { PANELS_CHANGED, announcePanels, clampPanelWidth } from "../../../shared/lib/panelWidth";

const RESIZE_STEP = 16;
const SIDEBAR_CSS_VAR = "--w-sidebar";
const SIDEBAR_FADE_OUT_MS = 100;

const detailsPanelWidth = () => {
  const slot = document.querySelector<HTMLElement>(".details-slot");
  if (!slot || getComputedStyle(slot).position === "absolute") return 0;
  return slot.querySelector<HTMLElement>(".details-panel")?.getBoundingClientRect().width ?? 0;
};
const SIDEBAR_ANIMATION_MS = 300;

const prefersMotion = () =>
  typeof window.matchMedia === "function" &&
  !window.matchMedia("(prefers-reduced-motion: reduce)").matches &&
  !document.documentElement.classList.contains("reduce-motion");

interface SidebarProps {
  communities: Community[];
  channelsByCommunity: Record<string, Channel[]>;
  expandedCommunityIds: Set<string>;
  selectedChannelId: string | null;
  joinedChannelId: string | null;
  selectedDmId: string | null;
  directs: DirectConversation[];
  hasMoreDirects?: boolean;
  onLoadMoreDirects?: () => void;
  onToggleCommunity: (communityId: string) => void;
  onSelectChannel: (community: Community, channel: Channel) => void;
  onSelectDm: (dm: DirectConversation) => void;
  onInviteCommunity: (community: Community) => void;
  onOpenCommunitySettings: (community: Community) => void;
  onAddChannel: (community: Community, kind: ChannelKind) => void;
  onDeleteChannel: (community: Community, channel: Channel) => void;
  onAddFriend: () => void;
  onCreateCommunity: () => void;
  onJoinCommunity: () => void;
  currentUserId?: string;
  voiceMembers?: SidebarVoiceMember[];
  voiceMembersByChannel?: Record<string, SidebarVoiceMember[]>;
  unreadByChannel?: Record<string, number>;
  unreadByDm?: Record<string, number>;
  channelToCommunity?: Record<string, string>;
  onLeaveCommunity?: (community: Community) => void;
  onCallDm?: (dm: DirectConversation) => void;
  nav: SidebarNav;
}

export interface SidebarNav {
  tab: "dms" | "communities";
  setTab: (tab: "dms" | "communities") => void;
  openCommunityId: string | null;
  setOpenCommunityId: (id: string | null) => void;
}

const Sidebar: React.FC<SidebarProps> = ({
  communities,
  channelsByCommunity,
  expandedCommunityIds,
  selectedChannelId,
  joinedChannelId,
  selectedDmId,
  directs,
  hasMoreDirects,
  onLoadMoreDirects,
  onToggleCommunity,
  onSelectChannel,
  onSelectDm,
  onInviteCommunity,
  onOpenCommunitySettings,
  onAddChannel,
  onDeleteChannel,
  onAddFriend,
  onCreateCommunity,
  onJoinCommunity,
  currentUserId,
  voiceMembers,
  voiceMembersByChannel,
  unreadByChannel = {},
  unreadByDm = {},
  channelToCommunity = {},
  onLeaveCommunity,
  onCallDm,
  nav,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const { tab, openCommunityId, setTab, setOpenCommunityId } = nav;
  const openCommunity = communities.find((s) => s.id === openCommunityId) ?? null;
  const perms = useSidebarPermissions(currentUserId, openCommunityId);
  const collapsed = useSidebarLayoutStore((s) => s.collapsed);
  const width = useSidebarLayoutStore((s) => s.width);
  const [roomyWidth, setRoomyWidth] = React.useState(width);
  const [flyoutId, setFlyoutId] = React.useState<string | null>(null);
  const flyoutRef = React.useRef<HTMLDivElement>(null);
  const [showRail, setShowRail] = React.useState(collapsed);
  const [phase, setPhase] = React.useState<"idle" | "out" | "in">("idle");
  const showRailRef = React.useRef(showRail);
  showRailRef.current = showRail;

  React.useEffect(() => {
    if (showRailRef.current === collapsed) {
      setPhase("idle");
      return;
    }
    if (!prefersMotion()) {
      setShowRail(collapsed);
      return;
    }
    setPhase("out");
    const swap = setTimeout(() => {
      setShowRail(collapsed);
      setPhase("in");
    }, SIDEBAR_FADE_OUT_MS);
    const done = setTimeout(() => setPhase("idle"), SIDEBAR_ANIMATION_MS);
    return () => {
      clearTimeout(swap);
      clearTimeout(done);
    };
  }, [collapsed]);
  const shownWidth = collapsed ? SIDEBAR_COLLAPSED_WIDTH : roomyWidth;
  const shownPhase = showRail !== collapsed && prefersMotion() ? "out" : phase;
  const animClass = shownPhase === "idle" ? "" : ` sb-anim sb-${shownPhase}`;
  const animStyle =
    shownPhase === "idle"
      ? undefined
      : ({
          "--sb-content-w": `${showRail ? SIDEBAR_COLLAPSED_WIDTH - 12 : roomyWidth - 16}px`,
        } as React.CSSProperties);

  React.useEffect(() => {
    const fit = () =>
      setRoomyWidth(
        clampPanelWidth(
          width,
          { min: SIDEBAR_MIN_WIDTH, max: SIDEBAR_MAX_WIDTH },
          window.innerWidth,
          detailsPanelWidth(),
        ),
      );
    const onPanels = (e: Event) => {
      if ((e as CustomEvent<string>).detail !== SIDEBAR_CSS_VAR) fit();
    };
    fit();
    window.addEventListener("resize", fit);
    window.addEventListener(PANELS_CHANGED, onPanels);
    return () => {
      window.removeEventListener("resize", fit);
      window.removeEventListener(PANELS_CHANGED, onPanels);
    };
  }, [width]);

  React.useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty(SIDEBAR_CSS_VAR, `${shownWidth}px`);
    announcePanels(SIDEBAR_CSS_VAR);
  }, [shownWidth]);

  React.useEffect(
    () => () => {
      document.documentElement.style.removeProperty(SIDEBAR_CSS_VAR);
      requestAnimationFrame(() => announcePanels(SIDEBAR_CSS_VAR));
    },
    [],
  );

  const collapse = () => {
    useSidebarLayoutStore.getState().setCollapsed(true);
  };
  const expand = () => useSidebarLayoutStore.getState().setCollapsed(false);

  const startResize = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const handle = e.currentTarget;
    const left = (handle.parentElement ?? handle).getBoundingClientRect().left;
    document.body.classList.add("sb-resizing");
    const move = (ev: PointerEvent) => useSidebarLayoutStore.getState().dragTo(ev.clientX - left);
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      window.removeEventListener("blur", stop);
      document.body.classList.remove("sb-resizing");
      if (document.activeElement instanceof HTMLElement && document.activeElement.classList.contains("sb-resizer")) {
        document.activeElement.blur();
      }
      useSidebarLayoutStore.getState().persist();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
    window.addEventListener("blur", stop);
  };

  const resizer = (
    <div
      className="sb-resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label={t("resizeSidebar")}
      aria-valuemin={SIDEBAR_COLLAPSED_WIDTH}
      aria-valuemax={SIDEBAR_MAX_WIDTH}
      aria-valuenow={shownWidth}
      tabIndex={0}
      onPointerDown={startResize}
      onDoubleClick={(e) => {
        e.currentTarget.blur();
        if (collapsed) expand();
        else collapse();
      }}
      onKeyDown={(e) => {
        if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
          e.preventDefault();
          useSidebarLayoutStore.getState().nudge(e.key === "ArrowLeft" ? -RESIZE_STEP : RESIZE_STEP);
        } else if (e.key === "Enter") {
          e.preventDefault();
          if (collapsed) expand();
          else collapse();
        }
      }}
    />
  );

  const dmPins = useConversationPinsStore((s) => s.dms);
  const hiddenDms = useHiddenDmStore((s) => s.hidden);
  const hiddenReady = useHiddenDmStore(hiddenDmsReady);
  const communityPins = useConversationPinsStore((s) => s.communities);
  const visibleDirects = pinnedFirst(directs, dmPins);
  const orderedCommunities = pinnedFirst(communities, communityPins);
  const channelsOf = (communityId: string) => channelsByCommunity[communityId] || [];

  const pickDm = (d: DirectConversation) => {
    onSelectDm(d);
  };

  const pickCommunity = (community: Community) => {
    if (!expandedCommunityIds.has(community.id)) onToggleCommunity(community.id);
    setOpenCommunityId(community.id);
  };

  const pickChannel = (community: Community, channel: Channel) => {
    setFlyoutId(null);
    onSelectChannel(community, channel);
    if (!expandedCommunityIds.has(community.id)) onToggleCommunity(community.id);
    setOpenCommunityId(community.id);
  };

  const totalDmUnread = hiddenReady
    ? Object.entries(unreadByDm).reduce((sum, [id, count]) => (hiddenDms.includes(id) ? sum : sum + count), 0)
    : 0;
  const totalCommunityUnread = Object.values(unreadByChannel).reduce(
    (a, b) => a + b,
    0,
  );

  const communityUnreadFor = (communityId: string) => {
    const all = channelsByCommunity[communityId] || [];
    const localSum = all
      .filter((c) => c.kind === "text")
      .reduce((sum, c) => sum + (unreadByChannel[c.id] ?? 0), 0);
    let fallbackSum = 0;
    for (const [cid, count] of Object.entries(unreadByChannel)) {
      if (channelToCommunity[cid] === communityId) fallbackSum += count;
    }
    return Math.max(localSum, fallbackSum);
  };

  const renderChannel = (community: Community, channel: Channel) => (
    <ChannelRow
      key={channel.id}
      community={community}
      channel={channel}
      perms={perms}
      selectedChannelId={selectedChannelId}
      joinedChannelId={joinedChannelId}
      unreadByChannel={unreadByChannel}
      voiceMembers={voiceMembers}
      voiceMembersByChannel={voiceMembersByChannel}
      currentUserId={currentUserId}
      onPick={pickChannel}
      onDeleteChannel={onDeleteChannel}
    />
  );

  const dmList = (
    <DmList
      directs={visibleDirects}
      selectedDmId={selectedDmId}
      unreadByDm={unreadByDm}
      hasMoreDirects={hasMoreDirects}
      onLoadMoreDirects={onLoadMoreDirects}
      onAddFriend={onAddFriend}
      onPick={pickDm}
      onCall={onCallDm}
    />
  );

  const communityView =
    openCommunity ? (
      <CommunityPage
        community={openCommunity}
        channels={channelsOf(openCommunity.id)}
        perms={perms}
        onInviteCommunity={onInviteCommunity}
        onOpenCommunitySettings={onOpenCommunitySettings}
        onAddChannel={onAddChannel}
        renderChannel={renderChannel}
      />
    ) : (
      <CommunityList
        communities={orderedCommunities}
        channelsByCommunity={channelsByCommunity}
        voiceMembersByChannel={voiceMembersByChannel}
        unreadFor={communityUnreadFor}
        onCreateCommunity={onCreateCommunity}
        onJoinCommunity={onJoinCommunity}
        onPick={pickCommunity}
        perms={perms}
        currentUserId={currentUserId}
        onInviteCommunity={onInviteCommunity}
        onOpenCommunitySettings={onOpenCommunitySettings}
        onLeaveCommunity={onLeaveCommunity}
      />
    );

  const flyoutCommunity = showRail ? (communities.find((c) => c.id === flyoutId) ?? null) : null;

  React.useEffect(() => {
    if (!flyoutCommunity) return;
    const onDoc = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (flyoutRef.current?.contains(target)) return;
      if (target.closest?.(`[data-community-id="${flyoutCommunity.id}"]`)) return;
      setFlyoutId(null);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setFlyoutId(null);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [flyoutCommunity]);

  const pickFromRail = (community: Community) => {
    if (flyoutId === community.id) {
      setFlyoutId(null);
      return;
    }
    pickCommunity(community);
    setFlyoutId(community.id);
  };

  if (showRail) {
    return (
      <div className={"sb sb-collapsed" + animClass} style={animStyle}>
        <div className="sb-rail">
          <button type="button" className="sb-rail-btn sb-rail-open" title={t("expandSidebar")} aria-label={t("expandSidebar")} onClick={expand}>
            <Icon name="panel-open" size={20} />
          </button>
          <div className="sb-rail-switch" role="tablist" aria-label={t("switchView")}>
            <button
              type="button"
              role="tab"
              aria-selected={tab === "dms"}
              className={"sb-rail-switch-btn" + (tab === "dms" ? " active" : "")}
              title={t("showDirects")}
              aria-label={t("showDirects")}
              onClick={() => setTab("dms")}
            >
              <Icon name="message" size={17} />
              {tab !== "dms" && totalDmUnread > 0 && <span className="sb-rail-dot" />}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={tab === "communities"}
              className={"sb-rail-switch-btn" + (tab === "communities" ? " active" : "")}
              title={t("showCommunities")}
              aria-label={t("showCommunities")}
              onClick={() => setTab("communities")}
            >
              <Icon name="users" size={17} />
              {tab !== "communities" && totalCommunityUnread > 0 && <span className="sb-rail-dot" />}
            </button>
          </div>
        </div>
        <div className="sb-rail-actions">
          {tab === "dms" ? (
            <>
              <button type="button" className="sb-mini sb-mini-more" title={t("addFriend")} aria-label={t("addFriend")} onClick={onAddFriend}>
                <Icon name="user-plus" size={17} />
              </button>
            </>
          ) : (
            <>
              <button type="button" className="sb-mini sb-mini-more" title={t("createCommunity")} aria-label={t("createCommunity")} onClick={onCreateCommunity}>
                <Icon name="plus" size={16} />
              </button>
              <button type="button" className="sb-mini sb-mini-more" title={t("joinCommunity")} aria-label={t("joinCommunity")} onClick={onJoinCommunity}>
                <Icon name="link" size={16} />
              </button>
            </>
          )}
        </div>
        <div className="sb-scroll sb-scroll-mini">
          {tab === "dms" ? (
            <DmList
              compact
              directs={visibleDirects}
              selectedDmId={selectedDmId}
              unreadByDm={unreadByDm}
              hasMoreDirects={hasMoreDirects}
              onLoadMoreDirects={onLoadMoreDirects}
              onAddFriend={onAddFriend}
              onPick={pickDm}
              onCall={onCallDm}
            />
          ) : (
            <CommunityList
              compact
              activeCommunityId={flyoutId ?? openCommunityId}
              communities={orderedCommunities}
              channelsByCommunity={channelsByCommunity}
              voiceMembersByChannel={voiceMembersByChannel}
              unreadFor={communityUnreadFor}
              onCreateCommunity={onCreateCommunity}
              onJoinCommunity={onJoinCommunity}
              onPick={pickFromRail}
              perms={perms}
              currentUserId={currentUserId}
              onInviteCommunity={onInviteCommunity}
              onOpenCommunitySettings={onOpenCommunitySettings}
              onLeaveCommunity={onLeaveCommunity}
            />
          )}
        </div>
        {flyoutCommunity && (
          <div className="sb-flyout" role="dialog" aria-label={flyoutCommunity.name} ref={flyoutRef}>
            <CommunityPage
              community={flyoutCommunity}
              channels={channelsOf(flyoutCommunity.id)}
              perms={perms}
              onInviteCommunity={onInviteCommunity}
              onOpenCommunitySettings={onOpenCommunitySettings}
              onAddChannel={onAddChannel}
              renderChannel={renderChannel}
            />
          </div>
        )}
        {resizer}
      </div>
    );
  }

  return (
    <div className={"sb" + animClass} style={animStyle}>
      <SidebarHeader
        tab={tab}
        onSetTab={setTab}
        openCommunityId={openCommunityId}
        onCloseCommunity={() => setOpenCommunityId(null)}
        dmDotVisible={totalDmUnread > 0}
        communityDotVisible={totalCommunityUnread > 0}
        onCollapse={collapse}
      />

      <div className="sb-scroll">
        {tab === "dms" ? (
          dmList
        ) : (
          communityView
        )}
      </div>

      {openCommunityId && (
        <button
          type="button"
          className="sb-add-row secondary sb-footer-back"
          onClick={() => setOpenCommunityId(null)}
        >
          <span className="sb-add-row-ico">
            <Icon name="chev-left" size={18} />
          </span>
          <span>{t("backToCommunities")}</span>
        </button>
      )}
      {resizer}
    </div>
  );
};

export default Sidebar;
