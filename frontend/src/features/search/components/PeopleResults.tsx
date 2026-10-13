import React from "react";
import { useTranslation } from "react-i18next";

import { UserAvatar } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import UserBadge from "../../subscription/components/UserBadge";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import { usePresenceStore } from "../../social/store/presenceStore";
import type { PersonRelation } from "../api/people";
import { usePeopleSearch } from "../hooks/usePeopleSearch";
import { highlightTerms } from "./GlobalSearchResults";

interface PeopleResultsProps {
  query: string;
  hiddenIds: ReadonlySet<string>;
  onMessage: (userId: string, name: string) => void;
  onDone: () => void;
}

const RELATION_LABEL: Record<PersonRelation, string> = {
  friend: "relationFriend",
  outgoing: "relationOutgoing",
  incoming: "relationIncoming",
  none: "relationNone",
};

const PeopleResults: React.FC<PeopleResultsProps> = ({ query, hiddenIds, onMessage, onDone }) => {
  const { t } = useTranslation("sidebar");
  const { people, loading, failed } = usePeopleSearch(query);
  const online = usePresenceStore((s) => s.online);
  const [busyId, setBusyId] = React.useState<string | null>(null);
  const [failedId, setFailedId] = React.useState<string | null>(null);
  const visible = people.filter((person) => !hiddenIds.has(person.id));

  if (!loading && !failed && visible.length === 0) return null;

  const act = (id: string, action: () => Promise<void>) => async () => {
    setBusyId(id);
    setFailedId(null);
    try {
      await action();
    } catch {
      setFailedId(id);
    } finally {
      setBusyId(null);
    }
  };

  return (
    <section className="search-dd-section" aria-label={t("searchPeople")}>
      <div className="search-dd-label">{t("searchPeople")}</div>
      {visible.length === 0 && (
        <div className="search-dd-note">{failed ? t("peopleFailed") : t("peopleSearching")}</div>
      )}
      {visible.map((person) => {
        const friendship = useFriendshipStore.getState();
        const busy = busyId === person.id;
        const sub =
          failedId === person.id
            ? t("peopleActionFailed")
            : person.relation === "none" && !person.acceptsFriendRequests
              ? t("requestsClosed")
              : t(RELATION_LABEL[person.relation]);
        const action =
          person.relation === "friend"
            ? {
                label: t("peopleMessage"),
                run: async () => {
                  onMessage(person.id, person.name);
                  onDone();
                },
              }
            : person.relation === "incoming"
              ? { label: t("peopleAccept"), run: () => friendship.accept(person.id) }
              : person.relation === "outgoing"
                ? { label: t("peopleCancel"), run: () => friendship.remove(person.id) }
                : person.acceptsFriendRequests
                  ? { label: t("peopleAdd"), run: () => friendship.sendRequest(person.name) }
                  : null;
        return (
          <div key={person.id} className="search-dd-person">
            <button
              type="button"
              className="search-dd-item"
              onClick={() => {
                useProfileModalStore.getState().open(person.id);
                onDone();
              }}
            >
              <span className="search-dd-ava-wrap">
                <UserAvatar
                  userId={person.id}
                  name={person.name}
                  className="search-dd-ava round"
                  style={{ background: colorForId(person.id) }}
                />
                {online[person.id] && <span className="search-dd-online" aria-hidden="true" />}
              </span>
              <span className="search-dd-text">
                <span className="search-dd-name">
                  {highlightTerms(person.name, query)}
                  <UserBadge userId={person.id} size="sm" />
                </span>
                <span className="search-dd-sub">{sub}</span>
              </span>
            </button>
            {action && (
              <button
                type="button"
                className="search-dd-action"
                disabled={busy}
                onClick={() => void act(person.id, action.run)()}
              >
                {action.label}
              </button>
            )}
          </div>
        );
      })}
    </section>
  );
};

export default PeopleResults;
