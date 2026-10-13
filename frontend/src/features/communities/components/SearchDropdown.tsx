import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { CommunityLogo, UserAvatar } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import type { Community } from "../../../shared/types/community";
import type { Channel } from "../../../shared/types/channel";
import type { DirectConversation } from "../../dm/types";
import type { OpenMessageLink } from "../../chat/lib/messageLink";
import GlobalSearchResults, { highlightTerms } from "../../search/components/GlobalSearchResults";
import InviteResult from "../../search/components/InviteResult";
import PeopleResults from "../../search/components/PeopleResults";
import { extractInviteToken } from "../../../shared/lib/inviteLinks";
import RecentSearches from "../../search/components/RecentSearches";
import { useSearchHistory } from "../../search/hooks/useSearchHistory";
import type { GlobalSearchHit } from "../../search/api/globalSearch";
import type { HistoryItem, HistoryTarget } from "../../search/api/searchHistory";
import { placeLink } from "../../search/components/GlobalSearchResults";
import { useProfileModalStore } from "../../social/store/profileModalStore";

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
  onOpenResult: (link: OpenMessageLink) => void;
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

  const people = directs.filter((d) => matches(d.name)).slice(0, QUICK_LIMIT);
  const groups = communities.filter((c) => matches(c.name)).slice(0, QUICK_LIMIT);
  const channels = communities
    .flatMap((community) =>
      (channelsByCommunity[community.id] ?? [])
        .filter((channel) => matches(channel.name))
        .map((channel) => ({ community, channel })),
    )
    .slice(0, QUICK_LIMIT);

  const inviteToken = extractInviteToken(query);
  const shownChatIds = new Set(people.map((d) => d.id));

  const choose = (action: () => void, target?: HistoryTarget) => () => {
    if (target) history.record(target);
    action();
    onDone();
  };

  const openRecent = (item: HistoryItem) => {
    if (item.type === "query") {
      history.record({ type: "query", text: item.text });
      onSearch(item.text);
      return;
    }
    history.record(item);
    switch (item.type) {
      case "person":
        useProfileModalStore.getState().open(item.userId);
        break;
      case "dm":
        onMessagePerson(item.userId, item.name, isFriend(item.userId));
        break;
      case "community": {
        const community = communities.find((c) => c.id === item.communityId);
        if (community) onPickCommunity(community);
        break;
      }
      case "channel": {
        const community = communities.find((c) => c.id === item.communityId);
        const channel = (channelsByCommunity[item.communityId] ?? []).find((c) => c.id === item.channelId);
        if (community && channel) onPickChannel(community, channel);
        else if (community) onPickCommunity(community);
        break;
      }
      case "message":
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
                  <span className="search-dd-name">{highlightTerms(d.name, query)}</span>
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
