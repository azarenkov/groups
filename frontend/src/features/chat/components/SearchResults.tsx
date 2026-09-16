import React from "react";

import i18n from "../../../i18n";
import type { ChannelMessage } from "../../../shared/types/channel";
import { UserAvatar } from "../../../shared/ui/Avatar";

interface SearchResultsProps {
  results: ChannelMessage[];
  currentUserId?: string;
  isDm: boolean;
  blockedIds: Record<string, unknown>;
  nameOverrides: Record<string, string>;
  revealedResults: Set<string>;
  onReveal: (messageId: string) => void;
  onJump: (message: ChannelMessage) => void;
}

const SearchResults: React.FC<SearchResultsProps> = ({
  results,
  currentUserId,
  isDm,
  blockedIds,
  nameOverrides,
  revealedResults,
  onReveal,
  onJump,
}) => (
  <>
    {results.map((m) => {
                  const isMine = !!currentUserId && m.authorId === currentUserId;
                  const hiddenAuthor =
                    !isDm && !!blockedIds[m.authorId] && !revealedResults.has(m.id);
                  const displayName = isMine
                    ? i18n.t("you", { ns: "chat" })
                    : (nameOverrides[m.authorId] ?? m.authorName ?? m.authorId);
                  const time = new Date(m.createdAt).toLocaleString([], {
                    month: "short",
                    day: "numeric",
                    hour: "2-digit",
                    minute: "2-digit",
                  });
                  return (
                    <button
                      key={m.id}
                      type="button"
                      className="search-result"
                      onClick={() => onJump(m)}
                    >
                      <UserAvatar
                        userId={m.authorId}
                        name={displayName}
                        className="ava"
                      />
                      <div className="body">
                        <div className="head">
                          <b>{displayName}</b>
                          <time>{time}</time>
                        </div>
                        {hiddenAuthor ? (
                          <div className="content msg-system-text">
                            {i18n.t("blockedMessage", { ns: "chat" })}{" "}
                            <span
                              role="button"
                              tabIndex={0}
                              className="link-btn"
                              onClick={(e) => {
                                e.stopPropagation();
                                onReveal(m.id);
                              }}
                              onKeyDown={(e) => {
                                if (e.key === "Enter" || e.key === " ") {
                                  e.preventDefault();
                                  e.stopPropagation();
                                  onReveal(m.id);
                                }
                              }}
                            >
                              {i18n.t("showBlockedMessage", { ns: "chat" })}
                            </span>
                          </div>
                        ) : (
                          <div className="content">{m.content}</div>
                        )}
                      </div>
                    </button>
                  );
                })}
  </>
);

export default SearchResults;
