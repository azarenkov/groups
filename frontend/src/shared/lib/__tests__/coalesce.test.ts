import { coalesce } from "../coalesce";

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("coalesce", () => {
  it("runs one call at a time and replays a single trailing call", async () => {
    const releases: Array<() => void> = [];
    let runs = 0;
    const run = coalesce(async () => {
      runs += 1;
      await new Promise<void>((resolve) => releases.push(resolve));
    });

    const first = run();
    void run();
    void run();
    await flush();
    expect(runs).toBe(1);

    releases[0]();
    await flush();
    expect(runs).toBe(2);

    releases[1]();
    await first;
    await flush();
    expect(runs).toBe(2);

    void run();
    await flush();
    expect(runs).toBe(3);
  });

  it("keeps working after a rejected run", async () => {
    let runs = 0;
    const run = coalesce(async () => {
      runs += 1;
      throw new Error("boom");
    });
    await run();
    await run();
    expect(runs).toBe(2);
  });
});
