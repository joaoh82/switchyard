import type { SessionRecord } from "@/lib/ipc";

/** "5 minutes ago", "yesterday" — coarse on purpose; nobody needs the seconds. */
export function timeAgo(then: number | null, now = Date.now()): string {
  if (then === null) return "";
  const minutes = Math.round((now - then) / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.round(hours / 24);
  return days === 1 ? "yesterday" : `${days} days ago`;
}

export function describeEnd(record: SessionRecord): string {
  if (record.running) return "running";
  if (record.interrupted) return "interrupted when Yardsort closed";
  if (record.exitCode === 0) return "ended";
  // 128 + n is a shell's way of saying "killed by signal n" — which is what closing a tab does.
  return (record.exitCode ?? 0) > 128 ? "stopped" : `exited with code ${record.exitCode}`;
}
