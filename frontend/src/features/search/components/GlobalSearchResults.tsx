import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { domainOf } from "../../../shared/lib/links";
import { formatFileSize } from "../../chat/lib/messageFormat";
import type { OpenMessageLink } from "../../chat/lib/messageLink";
import { formatListTime } from "../../dm/lib/directOrder";
import { useBlockStore } from "../../social/store/blockStore";
import { GLOBAL_SEARCH_KINDS, type GlobalSearchHit, type GlobalSearchKind, type GlobalSearchPlace } from "../api/globalSearch";
import { globalHitKey, useGlobalSearch } from "../hooks/useGlobalSearch";

interface GlobalSearchResultsProps {
  query: string;
  currentUserId?: string;
  onOpen: (link: OpenMessageLink, hit: GlobalSearchHit) => void;
}

const escapeRegExp = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

export function highlightTerms(text: string, query: string): React.ReactNode[] {
  const terms = query
    .split(/\s+/)
    .filter((term) => term.length > 0)
    .map(escapeRegExp);
  if (terms.length === 0) return [text];
  const pattern = new RegExp(`(${terms.join("|")})`, "giu");
  return text
    .split(pattern)
    .map((part, i) => (i % 2 === 1 ? <mark key={i}>{part}</mark> : <React.Fragment key={i}>{part}</React.Fragment>));
}

export const placeLink = (place: GlobalSearchPlace, messageId: string): OpenMessageLink =>
  place.type === "channel"
    ? { kind: "channel", communityId: place.communityId, channelId: place.channelId, messageId }
    : { kind: "dm", peerId: place.peerId, messageId, peerName: place.peerName || undefined };

export const hitLink = (hit: GlobalSearchHit): OpenMessageLink => placeLink(hit.place, hit.messageId);

const SearchHitRow: React.FC<{
  hit: GlobalSearchHit;
  kind: GlobalSearchKind;
  query: string;
  now: Date;
  mine: boolean;
  onOpen: (link: OpenMessageLink, hit: GlobalSearchHit) => void;
}> = ({ hit, kind, query, now, mine, onOpen }) => {
  const { t, i18n } = useTranslation(["sidebar", "chat"]);
  const blocked = useBlockStore((s) => !!s.blocked[hit.authorId] || (!!hit.forwardedAuthorId && !!s.blocked[hit.forwardedAuthorId]));
  const blocksLoaded = useBlockStore((s) => s.loaded);
  const hidden = hit.place.type === "channel" && (!blocksLoaded || blocked);
  const place =
    hit.place.type === "channel" ? `#${hit.place.channelName} · ${hit.place.communityName}` : hit.place.peerName;
  const author = mine ? t("chat:you") : hit.authorName || t("search.unknownAuthor");
  let body: React.ReactNode;
  if (hidden) {
    body = blocksLoaded ? <span className="sb-hit-muted">{t("chat:blockedMessage")}</span> : null;
  } else if (kind === "files" && hit.attachment) {
    body = (
      <>
        <Icon name="paperclip" size={12} />
        <span className="sb-hit-text">
          <span className="sb-hit-author">{author}: </span>
          {highlightTerms(hit.attachment.fileName, query)}
        </span>
        <span className="sb-hit-muted">{formatFileSize(hit.attachment.sizeBytes)}</span>
      </>
    );
  } else if (kind === "links" && hit.url) {
    body = (
      <>
        <Icon name="link" size={12} />
        <span className="sb-hit-text">
          <span className="sb-hit-author">{author}: </span>
          {highlightTerms(hit.url, query)}
        </span>
      </>
    );
  } else {
    body = (
      <span className="sb-hit-text">
        <span className="sb-hit-author">{author}: </span>
        {highlightTerms(hit.snippet, query)}
      </span>
    );
  }
  const open = () => onOpen(hitLink(hit), hit);
  return (
    <div
      role="button"
      tabIndex={0}
      className={"sb-hit sb-hit-" + kind}
      onClick={open}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          open();
        }
      }}
    >
      <div className="sb-hit-head">
        <span className="sb-hit-place">{place}</span>
        <time className="sb-hit-time" dateTime={hit.createdAt}>
          {formatListTime(hit.createdAt, now, i18n.language)}
        </time>
      </div>
      <div className="sb-hit-body">
        {body}
        {kind === "links" && hit.url && !hidden && (
          <a
            className="sb-hit-open"
            href={hit.url}
            target="_blank"
            rel="noopener noreferrer"
            title={t("search.openLink", { domain: domainOf(hit.url) })}
            aria-label={t("search.openLink", { domain: domainOf(hit.url) })}
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
          >
            <Icon name="globe" size={12} />
          </a>
        )}
      </div>
    </div>
  );
};

const GlobalSearchResults: React.FC<GlobalSearchResultsProps> = ({ query, currentUserId, onOpen }) => {
  const { t } = useTranslation("sidebar");
  const search = useGlobalSearch(query);
  const now = new Date();
  if (!search.active) return null;
  return (
    <div className="sb-global-search">
      {GLOBAL_SEARCH_KINDS.map((kind) => {
        const section = search.sections[kind];
        const pending = !search.settled || (section.loading && section.items.length === 0);
        return (
          <section className="sb-view" key={kind} aria-label={t(`search.${kind}`)}>
            <div className="sb-vhead">
              <span className="sb-vhead-label">{t(`search.${kind}`)}</span>
            </div>
            {section.items.map((hit) => (
              <SearchHitRow
                key={globalHitKey(hit)}
                hit={hit}
                kind={kind}
                query={query.trim()}
                now={now}
                mine={!!currentUserId && hit.authorId === currentUserId}
                onOpen={onOpen}
              />
            ))}
            {pending && (
              <div className="sb-hit-loading">
                <span className="loading-spinner" />
              </div>
            )}
            {search.settled && section.loaded && !section.loading && !section.failed && section.items.length === 0 && (
              <div className="sb-empty">{t("search.nothing")}</div>
            )}
            {search.settled && section.failed && (
              <div className="sb-empty sb-hit-error" role="alert">
                <span>{t("search.failed")}</span>
                <button type="button" className="sb-load-more" onClick={() => search.retry(kind)}>
                  {t("search.retry")}
                </button>
              </div>
            )}
            {search.settled && section.nextCursor && !section.failed && (
              <button
                type="button"
                className="sb-load-more"
                disabled={section.loading}
                onClick={() => search.loadMore(kind)}
              >
                {t("loadMore")}
              </button>
            )}
          </section>
        );
      })}
    </div>
  );
};

export default GlobalSearchResults;
