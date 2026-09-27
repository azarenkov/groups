import {
  PERMISSIONS,
  PERMISSION_LABEL_KEYS,
  isSubset,
  readPermissions,
  union,
  without,
} from "../permissions";

describe("readPermissions", () => {
  it("reads the name list and ignores names this client does not know", () => {
    expect(readPermissions(["kickMembers", "flyAround", "muteMembers"])).toEqual(["muteMembers", "kickMembers"]);
  });

  it("falls back to the old mask from a server without names", () => {
    expect(readPermissions(undefined, 8 | 256)).toEqual(["muteMembers", "disconnectMembers"]);
  });

  it("returns nothing for missing data", () => {
    expect(readPermissions(undefined)).toEqual([]);
  });
});

describe("permission sets", () => {
  it("combines and subtracts in a stable order", () => {
    expect(union(["kickMembers"], ["manageChannels", "kickMembers"])).toEqual(["manageChannels", "kickMembers"]);
    expect(without(["manageChannels", "kickMembers"], ["kickMembers"])).toEqual(["manageChannels"]);
    expect(isSubset(["muteMembers"], ["muteMembers", "kickMembers"])).toBe(true);
    expect(isSubset(["manageRoles"], ["muteMembers"])).toBe(false);
  });

  it("has a label for every permission", () => {
    for (const p of PERMISSIONS) expect(PERMISSION_LABEL_KEYS[p]).toBeTruthy();
  });
});
