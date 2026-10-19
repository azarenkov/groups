import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { CommunityLogo, UserAvatar } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import type { HistoryItem } from "../api/searchHistory";
import { useBlockStore } from "../../social/store/blockStore";
import UserEmojiStatus from "../../social/components/UserEmojiStatus";

interface RecentSearchesProps {
  items: HistoryItem[];
  onPick: (item: HistoryItem) => void;
  onRemove: (id: string) => void;
  onClear: () => void;
}


const RecentIcon: React.FC<{ item: HistoryItem; masked: boolean }> = ({ item, masked }) => {
  if (masked) {
    return (
      <span className="search-dd-ava search-dd-icon round">
        <Icon name="eye-off" size={16} />
      </span>
    );
  }
  switch (item.type) {
    case "query":
      return (
        <span className="search-dd-ava search-dd-icon round">
          <Icon name="search" size={16} />
        </span>
      );
    case "person":
    case "dm":
      return (
        <UserAvatar
          userId={item.userId}
          name={item.name}
          className="search-dd-ava round"
          style={{ background: colorForId(item.userId) }}
        />
      );
    case "community":
      return (
        <CommunityLogo
          communityId={item.communityId}
          name={item.name}
          className="search-dd-ava"
          style={{ background: colorForId(item.communityId) }}
        />
      );
    case "channel":
      return (
        <span className="search-dd-ava search-dd-icon">
          <Icon name={item.voice ? "volume" : "hash"} size={16} />
        </span>
      );
    case "message":
      return (
        <UserAvatar
          userId={item.authorId}
          name={item.authorName}
          className="search-dd-ava round"
          style={{ background: colorForId(item.authorId) }}
        />
      );
  }
};

const RecentSearches: React.FC<RecentSearchesProps> = ({ items, onPick, onRemove, onClear }) => {
  const { t } = useTranslation(["sidebar", "chat"]);
  const blocked = useBlockStore((s) => s.blocked);
  const blocksLoaded = useBlockStore((s) => s.loaded);
  const masked = (item: HistoryItem): boolean =>
    item.type === "message" &&
    item.place.type === "channel" &&
    (!blocksLoaded || !!blocked[item.authorId] || (!!item.forwardedAuthorId && !!blocked[item.forwardedAuthorId]));
  const label = (item: HistoryItem): string => {
    switch (item.type) {
      case "query":
        return item.text;
      case "message":
        return masked(item) ? t("chat:blockedMessage") : item.snippet || item.authorName;
      default:
        return item.name;
    }
  };
  const sub = (item: HistoryItem): string => {
    switch (item.type) {
      case "query":
        return t("recentQuery");
      case "person":
        return t("recentPerson");
      case "dm":
        return t("searchDirect");
      case "community":
        return t("searchCommunity");
      case "channel":
        return item.communityName;
      case "message":
        if (item.place.type !== "channel") return t("recentMessageFrom", { author: item.authorName });
        return masked(item)
          ? `#${item.place.channelName}`
          : t("recentMessageIn", { author: item.authorName, place: `#${item.place.channelName}` });
    }
  };

  if (items.length === 0) {
    return <div className="search-dd-note">{t("recentEmpty")}</div>;
  }

  return (
    <section className="search-dd-section" aria-label={t("recentTitle")}>
      <div className="search-dd-label">{t("recentTitle")}</div>
      {items.map((item) => (
        <div key={item.id} className="search-dd-person">
          <button type="button" className="search-dd-item" onClick={() => onPick(item)}>
            <RecentIcon item={item} masked={masked(item)} />
            <span className="search-dd-text">
              {item.type === "person" || item.type === "dm" ? (
                <span className="search-dd-name search-dd-name-badged">
                  <span className="search-dd-name-text">{label(item)}</span>
                  <UserEmojiStatus userId={item.userId} size="sm" />
                </span>
              ) : (
                <span className="search-dd-name">{label(item)}</span>
              )}
              <span className="search-dd-sub">{sub(item)}</span>
            </span>
          </button>
          <button
            type="button"
            data-search-manage=""
            className="search-dd-remove"
            title={t("recentRemove")}
            aria-label={t("recentRemove")}
            onClick={() => onRemove(item.id)}
          >
            <Icon name="x" size={12} />
          </button>
        </div>
      ))}
      <div className="search-dd-footer">
        <button type="button" data-search-manage="" className="search-dd-clear" onClick={onClear}>
          {t("recentClear")}
        </button>
      </div>
    </section>
  );
};

export default RecentSearches;
