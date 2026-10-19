import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import "../../../styles/home.css";

export interface HomeViewProps {
  chats: React.ReactNode;
  groups: React.ReactNode;
  canCreateGroups: boolean;
  onAddFriend: () => void;
  onCreateGroup: () => void;
  onJoinGroup: () => void;
}

const HomeView: React.FC<HomeViewProps> = ({ chats, groups, canCreateGroups, onAddFriend, onCreateGroup, onJoinGroup }) => {
  const { t } = useTranslation(["sidebar"]);
  return (
    <div className="home">
      <div className="home-board">
        <section className="home-col" aria-label={t("tabDirects")}>
          <header className="home-col-head">
            <h2 className="home-col-title">{t("tabDirects")}</h2>
            <button type="button" className="sb-rail-btn" title={t("addFriend")} aria-label={t("addFriend")} onClick={onAddFriend}>
              <Icon name="user-plus" size={18} />
            </button>
          </header>
          <div className="home-col-body sb-scroll">{chats}</div>
        </section>
        <section className="home-col" aria-label={t("tabCommunities")}>
          <header className="home-col-head">
            <h2 className="home-col-title">{t("tabCommunities")}</h2>
            <button type="button" className="sb-rail-btn" title={t("joinCommunity")} aria-label={t("joinCommunity")} onClick={onJoinGroup}>
              <Icon name="link" size={18} />
            </button>
            {canCreateGroups && (
              <button type="button" className="sb-rail-btn" title={t("createCommunity")} aria-label={t("createCommunity")} onClick={onCreateGroup}>
                <Icon name="plus" size={18} />
              </button>
            )}
          </header>
          <div className="home-col-body sb-scroll">{groups}</div>
        </section>
      </div>
    </div>
  );
};

export default HomeView;
