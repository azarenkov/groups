import { groupMembers } from "../memberOrder";

const m = (
  id: string,
  online: boolean,
  rolePosition: number | null = null,
  owner = false,
) => ({ id, name: id, online, rolePosition, owner });

const ids = (list: { id: string }[]) => list.map((x) => x.id);

describe("groupMembers", () => {
  it("puts people who are online first", () => {
    const groups = groupMembers([m("ann", false), m("bob", true), m("cid", false), m("dan", true)]);
    expect(ids(groups.online)).toEqual(["bob", "dan"]);
    expect(ids(groups.offline)).toEqual(["ann", "cid"]);
  });

  it("keeps the role order inside each group", () => {
    const groups = groupMembers([
      m("zed", true),
      m("mod", true, 5),
      m("admin", true, 10),
      m("owner", true, null, true),
      m("old-admin", false, 10),
      m("guest", false),
    ]);
    expect(ids(groups.online)).toEqual(["owner", "admin", "mod", "zed"]);
    expect(ids(groups.offline)).toEqual(["old-admin", "guest"]);
  });

  it("sorts one role by name without caring about case", () => {
    const groups = groupMembers([m("bob", true, 1), m("Alice", true, 1), m("carl", true, 1)]);
    expect(ids(groups.online)).toEqual(["Alice", "bob", "carl"]);
  });
});
