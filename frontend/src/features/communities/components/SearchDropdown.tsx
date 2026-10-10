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
  onDone: () => void;
}

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
  onDone,
}) => {
  const { t } = useTranslation("sidebar");
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

  const choose = (action: () => void) => () => {
    action();
    onDone();
  };

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
        {people.length > 0 && (
          <section className="search-dd-section" aria-label={t("searchPeople")}>
            <div className="search-dd-label">{t("searchPeople")}</div>
            {people.map((d) => (
              <button key={d.id} type="button" className="search-dd-item" onClick={choose(() => onPickDm(d))}>
                <UserAvatar userId={d.id} name={d.name} className="search-dd-ava round" style={{ background: d.color }} />
                <span className="search-dd-text">
                  <span className="search-dd-name">{highlightTerms(d.name, query)}</span>
                  <span className="search-dd-sub">{t("searchDirect")}</span>
                </span>
              </button>
            ))}
          </section>
        )}
        {groups.length > 0 && (
          <section className="search-dd-section" aria-label={t("searchCommunities")}>
            <div className="search-dd-label">{t("searchCommunities")}</div>
            {groups.map((c) => (
              <button key={c.id} type="button" className="search-dd-item" onClick={choose(() => onPickCommunity(c))}>
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
                onClick={choose(() => onPickChannel(community, channel))}
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
          onOpen={(link) => {
            onOpenResult(link);
            onDone();
          }}
        />
      </div>
    </div>
  );
};

export default SearchDropdown;
