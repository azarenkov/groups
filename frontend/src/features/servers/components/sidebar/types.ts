export interface SidebarVoiceMember {
  id: string;
  name: string;
  initials: string;
  color: string;
  isMuted?: boolean;
  mutedBy?: string;
  isDeafened?: boolean;
  deafenedBy?: string;
  isSharingScreen?: boolean;
  isCameraOn?: boolean;
}
