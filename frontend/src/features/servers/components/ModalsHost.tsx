import React from "react";
import { useTranslation } from "react-i18next";

import AddFriendModal from "../../social/components/AddFriendModal";
import CreateChannelModal from "./CreateChannelModal";
import CreateServerModal from "./CreateServerModal";
import InviteFriendsModal from "./InviteFriendsModal";
import PasswordPromptModal from "./PasswordPromptModal";
import PendingFriendsModal from "../../social/components/PendingFriendsModal";
import ActiveCallBar from "../../voice/components/ActiveCallBar";
import ConfirmModal from "../../../shared/ui/ConfirmModal";
import IncomingCallModal from "../../voice/components/IncomingCallModal";
import UserProfileModal from "../../social/components/UserProfileModal";
import ReportModal from "../../moderation/components/ReportModal";
import type { Channel, ChannelKind } from "../../../shared/types/channel";
import type { Friendship } from "../../../shared/types/friendship";
import type { Server } from "../../../shared/types/server";

export type DeleteConfirmTarget = {
  type: "server" | "leave-server" | "channel" | "friend" | "block";
  id: string;
  name: string;
};

interface ModalsHostProps {
  serverModal: {
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
  channelModal: { serverId: string; kind: ChannelKind } | null;
  onCloseChannelModal: () => void;
  onCreateChannel: (
    name: string,
    kind: ChannelKind,
    visibility: "public" | "private",
    password?: string,
  ) => Promise<void>;
  passwordPrompt: { server: Server; channel: Channel } | null;
  onClosePasswordPrompt: () => void;
  onSubmitPassword: (password: string) => void;
  isAddFriendOpen: boolean;
  onCloseAddFriend: () => void;
  isPendingOpen: boolean;
  onClosePending: () => void;
  onOpenAddFriend: () => void;
  inviteModalServer: Server | null;
  acceptedFriends: Friendship[];
  onCloseInviteModal: () => void;
  deleteConfirm: DeleteConfirmTarget | null;
  onCancelConfirm: () => void;
  onConfirm: () => void;
  onOpenDm: (friendId: string, name?: string) => void;
  onCall: (friendId: string, video?: boolean) => void;
  onRemoveFriend: (id: string, name: string) => void;
  onBlockUser: (id: string, name: string) => void;
  callBar: { dockId: string; onOpen: () => void };
}

const ModalsHost: React.FC<ModalsHostProps> = ({
  serverModal,
  channelModal,
  onCloseChannelModal,
  onCreateChannel,
  passwordPrompt,
  onClosePasswordPrompt,
  onSubmitPassword,
  isAddFriendOpen,
  onCloseAddFriend,
  isPendingOpen,
  onClosePending,
  onOpenAddFriend,
  inviteModalServer,
  acceptedFriends,
  onCloseInviteModal,
  deleteConfirm,
  onCancelConfirm,
  onConfirm,
  onOpenDm,
  onCall,
  onRemoveFriend,
  onBlockUser,
  callBar,
}) => {
  const { t: tModals } = useTranslation("modals");
  return (
    <>
      <CreateServerModal
        isOpen={serverModal.isOpen}
        onClose={serverModal.onClose}
        onCreate={serverModal.onCreate}
        initialView={serverModal.view}
        initialToken={serverModal.token}
        onJoinByLink={serverModal.onJoinByLink}
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
      />

      <AddFriendModal
        isOpen={isAddFriendOpen}
        onClose={onCloseAddFriend}
      />

      <PendingFriendsModal
        isOpen={isPendingOpen}
        onClose={onClosePending}
        onAddFriend={onOpenAddFriend}
      />

      <InviteFriendsModal
        server={inviteModalServer}
        friends={acceptedFriends}
        onClose={onCloseInviteModal}
      />

      {deleteConfirm && (
        <ConfirmModal
          title={
            deleteConfirm.type === "server"
              ? tModals("confirm.deleteServerTitle")
              : deleteConfirm.type === "leave-server"
                ? tModals("confirm.leaveServerTitle")
                : deleteConfirm.type === "channel"
                  ? tModals("confirm.deleteChannelTitle")
                  : deleteConfirm.type === "block"
                    ? tModals("confirm.blockUserTitle")
                    : tModals("confirm.removeFriendTitle")
          }
          subtitle={
            deleteConfirm.type === "leave-server"
              ? tModals("confirm.leaveServerSubtitle")
              : deleteConfirm.type === "friend"
                ? tModals("confirm.removeFriendSubtitle")
                : deleteConfirm.type === "block"
                  ? tModals("confirm.blockUserSubtitle")
                  : tModals("confirm.thisCantBeUndone")
          }
          targetName={deleteConfirm.name}
          confirmLabel={
            deleteConfirm.type === "leave-server"
              ? tModals("confirm.leave")
              : deleteConfirm.type === "friend"
                ? tModals("confirm.remove")
                : deleteConfirm.type === "block"
                  ? tModals("confirm.block")
                  : tModals("confirm.delete")
          }
          danger={deleteConfirm.type !== "leave-server"}
          onCancel={onCancelConfirm}
          onConfirm={onConfirm}
        />
      )}
      <ReportModal />
    </>
  );
};

export default ModalsHost;
