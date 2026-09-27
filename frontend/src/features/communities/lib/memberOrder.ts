export interface MemberOrderInput {
  id: string;
  name: string;
  online: boolean;
  owner: boolean;
  rolePosition: number | null;
}

export interface MemberGroups<T> {
  online: T[];
  offline: T[];
}

const rank = (m: MemberOrderInput): number =>
  m.owner ? Number.POSITIVE_INFINITY : (m.rolePosition ?? Number.NEGATIVE_INFINITY);

export function groupMembers<T extends MemberOrderInput>(members: T[]): MemberGroups<T> {
  const sorted = [...members].sort((a, b) => {
    const byRank = rank(b) - rank(a);
    if (byRank !== 0 && !Number.isNaN(byRank)) return byRank;
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
  return {
    online: sorted.filter((m) => m.online),
    offline: sorted.filter((m) => !m.online),
  };
}
