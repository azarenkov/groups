import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { CommunityLogo, UserAvatar } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import type { Community } from "../../../shared/types/community";
import type { Channel } from "../../../shared/types/channel";
import type { DirectConversation } from "../../dm/types";
import type { ChatLocation } from "../../navigation/lib/appLocation";
import GlobalSearchResults, { highlightTerms } from "../../search/components/GlobalSearchResults";
import InviteResult from "../../search/components/InviteResult";
import PeopleResults from "../../search/components/PeopleResults";
import { extractInviteToken, extractProfileLinkId } from "../../../shared/lib/inviteLinks";
import RecentSearches from "../../search/components/RecentSearches";
import { useSearchHistory } from "../../search/hooks/useSearchHistory";
import type { GlobalSearchHit } from "../../search/api/globalSearch";
import type { HistoryItem, HistoryTarget } from "../../search/api/searchHistory";
import { placeLink } from "../../search/components/GlobalSearchResults";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import UserEmojiStatus from "../../social/components/UserEmojiStatus";
import { useChannelStore } from "../store/channelStore";
import { useCommunityStore } from "../store/communityStore";
import { revalidateSearchHistory } from "../../search/lib/searchHistory";

const QUICK_LIMIT = 5;

interface SearchDropdownProps {
  query: string;
  currentUserId?: string;
  directs: DirectConversation[];
  communities: Community[];
  channelsByCommunity: Record<string, Channel[]>;
  onPickDm: (dm: DirectConversation) => void;
  onPickCommunity: (community: Community) => void;
  onPickChannel: (community: Community, channel: Channel) => void;
  onOpenResult: (link: ChatLocation) => void;
  onMessagePerson: (userId: string, name: string, friend: boolean) => void;
  onJoinInvite: (token: string) => Promise<void>;
  onSearch: (text: string) => void;
  isFriend: (userId: string) => boolean;
  onDone: () => void;
}

const messageTarget = (hit: GlobalSearchHit): HistoryTarget => ({
  type: "message",
  messageId: hit.messageId,
  authorId: hit.authorId,
  authorName: hit.authorName,
  forwardedAuthorId: hit.forwardedAuthorId,
  snippet: hit.snippet,
  createdAt: hit.createdAt,
  place: hit.place,
});

const channelTarget = (community: Community, channel: Channel): HistoryTarget => ({
  type: "channel",
  communityId: community.id,
  communityName: community.name,
  channelId: channel.id,
  name: channel.name,
  voice: channel.kind === "voice",
});

const SearchDropdown: React.FC<SearchDropdownProps> = ({
  query,
  currentUserId,
  directs,
  communities,
  channelsByCommunity,
  onPickDm,
  onPickCommunity,
  onPickChannel,
  onOpenResult,
  onMessagePerson,
  onJoinInvite,
  onSearch,
  isFriend,
  onDone,
}) => {
  const { t } = useTranslation("sidebar");
  const history = useSearchHistory(true);
  const needle = query.trim().toLowerCase();
  const matches = (name: string) => name.toLowerCase().includes(needle);

  const linkedProfileId = extractProfileLinkId(query);
  const people = directs.filter((d) => d.id === linkedProfileId || matches(d.name)).slice(0, QUICK_LIMIT);
  const groups = communities.filter((c) => matches(c.name)).slice(0, QUICK_LIMIT);
  const channels = communities
    .flatMap((community) =>
      (channelsByCommunity[community.id] ?? [])
        .filter((channel) => matches(channel.name))
        .map((channel) => ({ community, channel })),
    )
    .slice(0, QUICK_LIMIT);

  const inviteToken = linkedProfileId ? null : extractInviteToken(query);
  const shownChatIds = new Set(people.map((d) => d.id));

  const choose = (action: () => void, target?: HistoryTarget) => () => {
    if (target) history.record(target);
    action();
    onDone();
  };

  const resolveCommunity = async (communityId: string): Promise<Community | undefined> => {
    const known = communities.find((c) => c.id === communityId);
    if (known) return known;
    await useCommunityStore.getState().fetchCommunities().catch(() => {});
    return useCommunityStore.getState().communities.find((c) => c.id === communityId);
  };

  const resolveChannel = async (communityId: string, channelId: string): Promise<Channel | undefined> => {
    const known = (channelsByCommunity[communityId] ?? []).find((c) => c.id === channelId);
    if (known) return known;
    await useChannelStore.getState().fetchChannels(communityId).catch(() => false);
    return (useChannelStore.getState().channelsByCommunity[communityId] ?? []).find((c) => c.id === channelId);
  };

  const openRecentPlace = async (item: Extract<HistoryItem, { type: "community" | "channel" }>) => {
    const community = await resolveCommunity(item.communityId);
    if (!community) {
      revalidateSearchHistory();
      return;
    }
    history.record(item);
    if (item.type === "channel") {
      const channel = await resolveChannel(item.communityId, item.channelId);
      if (channel) onPickChannel(community, channel);
      else onPickCommunity(community);
    } else {
      onPickCommunity(community);
    }
    onDone();
  };

  const openRecent = (item: HistoryItem) => {
    switch (item.type) {
      case "query":
        history.record({ type: "query", text: item.text });
        onSearch(item.text);
        return;
      case "community":
      case "channel":
        void openRecentPlace(item);
        return;
      case "person":
        history.record(item);
        useProfileModalStore.getState().open(item.userId);
        break;
      case "dm":
        history.record(item);
        onMessagePerson(item.userId, item.name, isFriend(item.userId));
        break;
      case "message":
        history.record(item);
        onOpenResult(placeLink(item.place, item.messageId));
        break;
    }
    onDone();
  };

  if (!needle) {
    return (
      <div className="search-dd">
        <div className="search-dd-list">
          <RecentSearches items={history.items} onPick={openRecent} onRemove={history.remove} onClear={history.clear} />
        </div>
      </div>
    );
  }

  return (
    <div className="search-dd">
      <div className="search-dd-hint" aria-hidden="true">
        <kbd>↑</kbd>
        <kbd>↓</kbd>
        <span>{t("searchNavigate")}</span>
        <span className="search-dd-hint-gap" />
        <kbd>Enter</kbd>
        <span>{t("searchOpen")}</span>
      </div>
      <div className="search-dd-list">
        {inviteToken && <InviteResult token={inviteToken} onJoin={onJoinInvite} onDone={onDone} />}
        {people.length > 0 && (
          <section className="search-dd-section" aria-label={t("searchChats")}>
            <div className="search-dd-label">{t("searchChats")}</div>
            {people.map((d) => (
              <button key={d.id} type="button" className="search-dd-item" onClick={choose(() => onPickDm(d), { type: "dm", userId: d.id, name: d.name })}>
                <UserAvatar userId={d.id} name={d.name} className="search-dd-ava round" style={{ background: d.color }} />
                <span className="search-dd-text">
                  <span className="search-dd-name search-dd-name-badged">
                    <span className="search-dd-name-text">{highlightTerms(d.name, query)}</span>
                    <UserEmojiStatus userId={d.id} size="sm" />
                  </span>
                  <span className="search-dd-sub">{t("searchDirect")}</span>
                </span>
              </button>
            ))}
          </section>
        )}
        <PeopleResults
          query={query}
          hiddenIds={shownChatIds}
          onMessage={(userId, name) => onMessagePerson(userId, name, true)}
          onRecord={history.record}
          onDone={onDone}
        />
        {groups.length > 0 && (
          <section className="search-dd-section" aria-label={t("searchCommunities")}>
            <div className="search-dd-label">{t("searchCommunities")}</div>
            {groups.map((c) => (
              <button key={c.id} type="button" className="search-dd-item" onClick={choose(() => onPickCommunity(c), { type: "community", communityId: c.id, name: c.name })}>
                <CommunityLogo
                  communityId={c.id}
                  name={c.name}
                  className="search-dd-ava"
                  style={{ background: c.accentColor ?? colorForId(c.id) }}
                />
                <span className="search-dd-text">
                  <span className="search-dd-name">{highlightTerms(c.name, query)}</span>
                  <span className="search-dd-sub">{t("searchCommunity")}</span>
                </span>
              </button>
            ))}
          </section>
        )}
        {channels.length > 0 && (
          <section className="search-dd-section" aria-label={t("searchChannels")}>
            <div className="search-dd-label">{t("searchChannels")}</div>
            {channels.map(({ community, channel }) => (
              <button
                key={channel.id}
                type="button"
                className="search-dd-item"
                onClick={choose(() => onPickChannel(community, channel), channelTarget(community, channel))}
              >
                <span className="search-dd-ava search-dd-icon">
                  <Icon name={channel.kind === "voice" ? "volume" : "hash"} size={16} />
                </span>
                <span className="search-dd-text">
                  <span className="search-dd-name">{highlightTerms(channel.name, query)}</span>
                  <span className="search-dd-sub">{community.name}</span>
                </span>
              </button>
            ))}
          </section>
        )}
        <GlobalSearchResults
          query={query}
          currentUserId={currentUserId}
          onOpen={(link, hit) => {
            history.record({ type: "query", text: query.trim() });
            history.record(messageTarget(hit));
            onOpenResult(link);
            onDone();
          }}
          onVisitLink={(hit) => {
            history.record({ type: "query", text: query.trim() });
            history.record(messageTarget(hit));
          }}
        />
      </div>
    </div>
  );
};

export default SearchDropdown;
