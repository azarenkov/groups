import React from "react";

import StickerImage from "../../stickers/components/StickerImage";
import { useActiveEmojiStatus } from "../lib/emojiStatus";
import type { EmojiStatus } from "../lib/emojiStatus";

const SIZES = { sm: 18, md: 22, lg: 28 } as const;

interface EmojiStatusIconProps {
  status: EmojiStatus | null | undefined;
  size?: keyof typeof SIZES;
}

const EmojiStatusIcon: React.FC<EmojiStatusIconProps> = ({ status, size = "sm" }) => {
  const active = useActiveEmojiStatus(status);
  if (!active) return null;
  return (
    <span className="emoji-status" data-testid="emoji-status">
      <StickerImage id={active.stickerId} emoji="" size={SIZES[size]} animated />
    </span>
  );
};

export default EmojiStatusIcon;
