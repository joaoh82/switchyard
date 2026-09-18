import type { Activity } from "./activity";

/**
 * What is going on somewhere, at a glance:
 * pulsing = printing right now · solid = running, quiet (probably waiting for you) ·
 * ringed = finished a long stretch of work you have not looked at yet · red = exited with an error.
 */
export function StatusDot({ activity, attention }: { activity: Activity; attention?: boolean }) {
  const colour = {
    busy: "bg-accent animate-pulse",
    waiting: "bg-accent",
    idle: "bg-line",
    failed: "bg-red-400",
  }[activity];
  const label = attention ? "finished, not seen yet" : activity;
  return (
    <span
      role="img"
      aria-label={label}
      title={label}
      className={`size-1.5 shrink-0 rounded-full ${colour} ${
        attention ? "ring-2 ring-accent/60 ring-offset-1 ring-offset-surface" : ""
      }`}
    />
  );
}
