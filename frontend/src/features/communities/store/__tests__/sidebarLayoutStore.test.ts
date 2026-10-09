import {
  SIDEBAR_MAX_WIDTH,
  SIDEBAR_MIN_WIDTH,
  useSidebarLayoutStore,
} from "../sidebarLayoutStore";

beforeEach(() => {
  window.localStorage.clear();
  useSidebarLayoutStore.setState({ width: 320, collapsed: false });
});

it("snaps to the collapsed rail below the threshold and clamps the expanded width", () => {
  const store = useSidebarLayoutStore.getState();
  store.dragTo(120);
  expect(useSidebarLayoutStore.getState().collapsed).toBe(true);
  store.dragTo(200);
  expect(useSidebarLayoutStore.getState()).toMatchObject({ collapsed: false, width: SIDEBAR_MIN_WIDTH });
  store.dragTo(900);
  expect(useSidebarLayoutStore.getState().width).toBe(SIDEBAR_MAX_WIDTH);
});

it("steps with the keyboard and collapses from the narrowest width", () => {
  useSidebarLayoutStore.setState({ width: SIDEBAR_MIN_WIDTH, collapsed: false });
  useSidebarLayoutStore.getState().nudge(-16);
  expect(useSidebarLayoutStore.getState().collapsed).toBe(true);
  useSidebarLayoutStore.getState().nudge(16);
  expect(useSidebarLayoutStore.getState()).toMatchObject({ collapsed: false, width: SIDEBAR_MIN_WIDTH });
  useSidebarLayoutStore.getState().nudge(16);
  expect(useSidebarLayoutStore.getState().width).toBe(SIDEBAR_MIN_WIDTH + 16);
});

it("remembers the width and mode", () => {
  useSidebarLayoutStore.getState().dragTo(360);
  useSidebarLayoutStore.getState().persist();
  useSidebarLayoutStore.getState().setCollapsed(true);
  expect(JSON.parse(window.localStorage.getItem("rift.sidebarLayout") ?? "{}")).toEqual({ width: 360, collapsed: true });
});

it("picks up the width saved by the old sidebar resizer", () => {
  window.localStorage.setItem("rift.panel.sidebar", "380");
  jest.isolateModules(() => {
    const { useSidebarLayoutStore: fresh } = require("../sidebarLayoutStore");
    expect(fresh.getState()).toMatchObject({ width: 380, collapsed: false });
  });
});
