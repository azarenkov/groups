import React from "react";
import { useTranslation } from "react-i18next";

import CreateChannelModal from "./CreateChannelModal";
import CreateCommunityModal from "./CreateCommunityModal";
import InviteFriendsModal from "./InviteFriendsModal";
import PasswordPromptModal from "./PasswordPromptModal";
import PendingFriendsModal from "../../social/components/PendingFriendsModal";
import ActiveCallBar from "../../voice/components/ActiveCallBar";
import ConfirmModal from "../../../shared/ui/ConfirmModal";
import IncomingCallModal from "../../voice/components/IncomingCallModal";
import UserProfileModal from "../../social/components/UserProfileModal";
import ReportModal from "../../moderation/components/ReportModal";
import ForwardModal from "../../chat/components/ForwardModal";
import type { Channel, ChannelKind } from "../../../shared/types/channel";
import type { Friendship } from "../../../shared/types/friendship";
import type { Community } from "../../../shared/types/community";

export type DeleteConfirmTarget = {
  type: "community" | "leave-community" | "channel" | "friend" | "block";
  id: string;
  name: string;
};

interface ModalsHostProps {
  communityModal: {
    isOpen: boolean;
    view: "create" | "join" | "choose";
    token: string | null;
    onClose: () => void;
    onCreate: (
      name: string,
      description?: string,
      accentColor?: string,
      logoFile?: File,
    ) => Promise<void | { logoError?: string }>;
    onJoinByLink: (token: string) => Promise<void>;
  };
  channelModal: { communityId: string; kind: ChannelKind } | null;
  onCloseChannelModal: () => void;
  onCreateChannel: (
    name: string,
    kind: ChannelKind,
    visibility: "public" | "private",
    password?: string,
  ) => Promise<void>;
  passwordPrompt: { community: Community; channel: Channel } | null;
  onClosePasswordPrompt: () => void;
  onSubmitPassword: (password: string) => void;
  isPendingOpen: boolean;
  onClosePending: () => void;
  onOpenAddFriend: () => void;
  inviteModalCommunity: Community | null;
  acceptedFriends: Friendship[];
  onCloseInviteModal: () => void;
  deleteConfirm: DeleteConfirmTarget | null;
  onCancelConfirm: () => void;
  onConfirm: () => void;
  onOpenDm: (friendId: string, name?: string) => void;
  onCall: (friendId: string, video?: boolean) => void;
  onRemoveFriend: (id: string, name: string) => void;
  onBlockUser: (id: string, name: string) => void;
  onEditProfile: () => void;
  callBar: { dockId: string; onOpen: () => void };
}

const ModalsHost: React.FC<ModalsHostProps> = ({
  communityModal,
  channelModal,
  onCloseChannelModal,
  onCreateChannel,
  passwordPrompt,
  onClosePasswordPrompt,
  onSubmitPassword,
  isPendingOpen,
  onClosePending,
  onOpenAddFriend,
  inviteModalCommunity,
  acceptedFriends,
  onCloseInviteModal,
  deleteConfirm,
  onCancelConfirm,
  onConfirm,
  onOpenDm,
  onCall,
  onRemoveFriend,
  onBlockUser,
  onEditProfile,
  callBar,
}) => {
  const { t: tModals } = useTranslation("modals");
  return (
    <>
      <CreateCommunityModal
        isOpen={communityModal.isOpen}
        onClose={communityModal.onClose}
        onCreate={communityModal.onCreate}
        initialView={communityModal.view}
        initialToken={communityModal.token}
        onJoinByLink={communityModal.onJoinByLink}
      />

      <CreateChannelModal
        isOpen={!!channelModal}
        defaultKind={channelModal?.kind}
        onClose={onCloseChannelModal}
        onCreate={onCreateChannel}
      />

      <PasswordPromptModal
        isOpen={!!passwordPrompt}
        channelName={passwordPrompt?.channel.name ?? ""}
        onClose={onClosePasswordPrompt}
        onSubmit={onSubmitPassword}
      />


      <IncomingCallModal />
      <ActiveCallBar
        dockId={callBar.dockId}
        onOpen={callBar.onOpen}
      />

      <UserProfileModal
        onOpenDm={onOpenDm}
        onCall={onCall}
        onRemoveFriend={onRemoveFriend}
        onBlockUser={onBlockUser}
        onEditProfile={onEditProfile}
      />

      <PendingFriendsModal
        isOpen={isPendingOpen}
        onClose={onClosePending}
        onAddFriend={onOpenAddFriend}
      />

      <InviteFriendsModal
        community={inviteModalCommunity}
        friends={acceptedFriends}
        onClose={onCloseInviteModal}
      />

      {deleteConfirm && (
        <ConfirmModal
          title={
            deleteConfirm.type === "community"
              ? tModals("confirm.deleteCommunityTitle")
              : deleteConfirm.type === "leave-community"
                ? tModals("confirm.leaveCommunityTitle")
                : deleteConfirm.type === "channel"
                  ? tModals("confirm.deleteChannelTitle")
                  : deleteConfirm.type === "block"
                    ? tModals("confirm.blockUserTitle")
                    : tModals("confirm.removeFriendTitle")
          }
          subtitle={
            deleteConfirm.type === "leave-community"
              ? tModals("confirm.leaveCommunitySubtitle")
              : deleteConfirm.type === "friend"
                ? tModals("confirm.removeFriendSubtitle")
                : deleteConfirm.type === "block"
                  ? tModals("confirm.blockUserSubtitle")
                  : tModals("confirm.thisCantBeUndone")
          }
          targetName={deleteConfirm.name}
          confirmLabel={
            deleteConfirm.type === "leave-community"
              ? tModals("confirm.leave")
              : deleteConfirm.type === "friend"
                ? tModals("confirm.remove")
                : deleteConfirm.type === "block"
                  ? tModals("confirm.block")
                  : tModals("confirm.delete")
          }
          danger={deleteConfirm.type !== "leave-community"}
          onCancel={onCancelConfirm}
          onConfirm={onConfirm}
        />
      )}
      <ReportModal />
      <ForwardModal />
    </>
  );
};

export default ModalsHost;
