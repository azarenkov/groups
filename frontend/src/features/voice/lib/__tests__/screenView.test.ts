import { LiveKitService } from "../livekit";

describe("screen view lifetime", () => {
  it("keeps the choice while a viewing surface is open", () => {
    const service = new LiveKitService();
    const release = service.acquireScreenView();
    service.watchScreen("ann");
    expect(service.getScreenShares().watching).toBe("ann");

    release();
    expect(service.getScreenShares().watching).toBeNull();
  });

  it("holds the choice until the last surface closes", () => {
    const service = new LiveKitService();
    const first = service.acquireScreenView();
    const second = service.acquireScreenView();
    service.watchScreen("bob");

    first();
    expect(service.getScreenShares().watching).toBe("bob");

    second();
    expect(service.getScreenShares().watching).toBeNull();
  });

  it("ignores a surface released twice", () => {
    const service = new LiveKitService();
    const first = service.acquireScreenView();
    const second = service.acquireScreenView();
    service.watchScreen("carol");

    first();
    first();
    expect(service.getScreenShares().watching).toBe("carol");

    second();
    expect(service.getScreenShares().watching).toBeNull();
  });

  it("watches several screens at once and promotes the next one when the first closes", () => {
    const service = new LiveKitService();
    const release = service.acquireScreenView();
    service.setScreenWatched("ann", true);
    service.setScreenWatched("bob", true);
    service.setScreenWatched("bob", true);
    expect(service.getScreenShares().watched).toEqual(["ann", "bob"]);

    service.setScreenWatched("ann", false);
    expect(service.getScreenShares().watching).toBe("bob");
    expect(service.getScreenShares().watched).toEqual(["bob"]);

    release();
    expect(service.getScreenShares().watched).toEqual([]);
  });

  it("switching to one screen drops the others", () => {
    const service = new LiveKitService();
    service.setScreenWatched("ann", true);
    service.setScreenWatched("bob", true);
    service.watchScreen("carol");
    expect(service.getScreenShares().watched).toEqual(["carol"]);
  });
});
