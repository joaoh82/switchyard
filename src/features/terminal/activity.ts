export type Activity = "busy" | "waiting" | "idle" | "failed";

/** Roll several terminals up into one dot: busy beats waiting beats idle. */
export function summarise(tabs: { busy: boolean; exit: unknown }[]): Activity {
  const live = tabs.filter((tab) => !tab.exit);
  if (live.some((tab) => tab.busy)) return "busy";
  return live.length > 0 ? "waiting" : "idle";
}
