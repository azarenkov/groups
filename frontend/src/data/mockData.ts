export interface MockDirect {
  id: string;
  name: string;
  initials: string;
  color: string;
  online: boolean;
  unread: number;
  lastMessage?: string;
}

export interface MockMember {
  id: string;
  name: string;
  initials: string;
  color: string;
  role: string;
  status: "online" | "idle" | "dnd" | "offline";
}

export const SEED_DIRECTS: MockDirect[] = [
  { id: "dm-maya", name: "Maya Lin", initials: "M", color: "#0071e3", online: true, unread: 0 },
  { id: "dm-theo", name: "Theo Rivers", initials: "T", color: "#ff6b35", online: true, unread: 1 },
  { id: "dm-ines", name: "Inés Park", initials: "I", color: "#bf5af2", online: false, unread: 0 },
];

export const SEED_MEMBERS: MockMember[] = [
  { id: "u-maya", name: "Maya Lin", initials: "M", color: "#0071e3", role: "Owner", status: "online" },
  { id: "u-theo", name: "Theo Rivers", initials: "T", color: "#ff6b35", role: "Designer", status: "online" },
  { id: "u-ines", name: "Inés Park", initials: "I", color: "#bf5af2", role: "Engineer", status: "idle" },
  { id: "u-noah", name: "Noah Kessler", initials: "N", color: "#30d158", role: "PM", status: "dnd" },
  { id: "u-sara", name: "Sara Chen", initials: "S", color: "#ff9f0a", role: "Engineer", status: "online" },
  { id: "u-arda", name: "Arda Yıldız", initials: "A", color: "#5b6cd9", role: "Member", status: "offline" },
  { id: "u-mira", name: "Mira Volkov", initials: "M", color: "#d97557", role: "Member", status: "offline" },
];

export { colorForId as colorFor } from "../utils/avatar";
