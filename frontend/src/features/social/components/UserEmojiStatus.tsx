import React from "react";

import { useEmojiStatusStore } from "../store/emojiStatusStore";
import EmojiStatusIcon from "./EmojiStatusIcon";

interface UserEmojiStatusProps {
  userId: string | null | undefined;
  size?: "sm" | "md" | "lg";
}

const UserEmojiStatus: React.FC<UserEmojiStatusProps> = ({ userId, size }) => {
  const status = useEmojiStatusStore((s) => (userId ? s.byUser[userId] : undefined));
  if (!status) return null;
  return <EmojiStatusIcon status={status} size={size} />;
};

export default UserEmojiStatus;
