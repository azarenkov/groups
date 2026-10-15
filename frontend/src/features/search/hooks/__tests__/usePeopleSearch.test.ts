import { normalizePeopleQuery } from "../usePeopleSearch";

it("searches people by the id in a pasted profile link", () => {
  const id = "60468fcd-88b9-4633-902c-3124191a8505";
  expect(normalizePeopleQuery(`https://rift.localtest.me/u/${id}`)).toBe(id);
});

it("keeps searching names by their folded text", () => {
  expect(normalizePeopleQuery("  AlEx ")).toBe("alex");
  expect(normalizePeopleQuery("a")).toBeNull();
});
