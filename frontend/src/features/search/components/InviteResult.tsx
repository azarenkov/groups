import React from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";

import { CommunityLogo } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import { communityAPI } from "../../communities/api/community";

interface InviteResultProps {
  token: string;
  onJoin: (token: string) => Promise<void>;
  onDone: () => void;
}

const InviteResult: React.FC<InviteResultProps> = ({ token, onJoin, onDone }) => {
  const { t } = useTranslation("sidebar");
  const preview = useQuery(
    {
      queryKey: queryKeys.invitePreview(token),
      queryFn: () => communityAPI.previewInvite(token),
      retry: false,
      staleTime: 0,
      refetchOnMount: "always",
    },
    queryClient,
  );
  const [joining, setJoining] = React.useState(false);
  const [joinFailed, setJoinFailed] = React.useState(false);
  const fresh = preview.isFetchedAfterMount && !preview.isFetching;
  const community = fresh && preview.data?.status === "active" ? preview.data.community : null;

  const join = async () => {
    setJoining(true);
    setJoinFailed(false);
    try {
      await onJoin(token);
      onDone();
    } catch {
      setJoinFailed(true);
      void preview.refetch();
    } finally {
      setJoining(false);
    }
  };

  return (
    <section className="search-dd-section" aria-label={t("searchInvite")}>
      <div className="search-dd-label">{t("searchInvite")}</div>
      {community ? (
        <div className="search-dd-person">
          <div className="search-dd-item search-dd-static">
            <CommunityLogo
              communityId={community.id}
              name={community.name}
              className="search-dd-ava"
              style={{ background: community.accentColor ?? colorForId(community.id) }}
            />
            <span className="search-dd-text">
              <span className="search-dd-name">{community.name}</span>
              <span className="search-dd-sub">
                {joinFailed ? t("inviteJoinFailed") : t("inviteMembers", { count: community.memberCount })}
              </span>
            </span>
          </div>
          <button type="button" className="search-dd-action" disabled={joining} onClick={() => void join()}>
            {t("inviteJoin")}
          </button>
        </div>
      ) : (
        <div className="search-dd-note">{fresh ? t("inviteUnavailable") : t("inviteChecking")}</div>
      )}
    </section>
  );
};

export default InviteResult;
