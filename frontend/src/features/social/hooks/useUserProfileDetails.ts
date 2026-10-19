import { useQuery } from "@tanstack/react-query";
import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import { parseEmojiStatus } from "../lib/emojiStatus";
import { useEmojiStatusStore } from "../store/emojiStatusStore";
import { homeAPI, type PublicProfile } from "../../home/api/home";
import { useAuthStore } from "../../auth/store/authStore";
import { useFriendshipStore } from "../store/friendshipStore";
import { blockGeneration, useBlockStore } from "../store/blockStore";
import { profileVersion, useUserProfileStore } from "../store/userProfileStore";

export function useUserProfileDetails(userId: string | null) {
  const friendshipStatus = useFriendshipStore((s) => (userId ? s.statusById[userId] : undefined));
  const blockedByMe = useBlockStore((s) => (userId ? !!s.blocked[userId] : false));
  const blockedMe = useBlockStore((s) => (userId ? !!s.blockedBy[userId] : false));
  const cachedBio = useUserProfileStore((s) => (userId ? s.bios[userId]?.bio : undefined));

  const profileQuery = useQuery(
    {
      queryKey: [...queryKeys.publicProfile(userId ?? ""), friendshipStatus ?? null, blockedByMe, blockedMe],
      queryFn: async () => {
        const id = userId as string;
        const gen = blockGeneration(id);
        const session = useAuthStore.getState().user?.id;
        const p = await homeAPI.profile(id);
        if (!session || useAuthStore.getState().user?.id !== session) throw new Error("SESSION_CHANGED");
        useUserProfileStore.getState().apply(id, p.bio, profileVersion(p.profileUpdatedAt));
        if (p.emojiStatus !== undefined) {
          useEmojiStatusStore.getState().apply(id, parseEmojiStatus(p.emojiStatus), profileVersion(p.profileUpdatedAt));
        }
        useBlockStore.getState().applyRelation(id, { blocked: p.blocked, blockedBy: p.blockedBy }, gen);
        return p;
      },
      enabled: !!userId,
      staleTime: 0,
    },
    queryClient,
  );
  const profile: PublicProfile | null = userId ? (profileQuery.data ?? null) : null;
  const failed = profileQuery.isError && !profile;
  const bio = blockedMe ? "" : (cachedBio ?? profile?.bio ?? "");

  return { profile, failed, blockedByMe, blockedMe, friendshipStatus, bio };
}
