import { useEffect } from "react";
import { useAppStore } from "@/stores/app";
import { CHECK_EVERY_MS, FIRST_CHECK_MS, useUpdatesStore } from "@/stores/updates";

/** Look for updates shortly after start and once a day — if the user has not switched it off. */
export function useUpdateChecks() {
  const enabled = useAppStore((s) => s.checkForUpdates);
  const isDevBuild = useAppStore((s) => s.info?.debug ?? true);

  useEffect(() => {
    if (!enabled || isDevBuild) return;
    const check = () => void useUpdatesStore.getState().check();
    const first = setTimeout(check, FIRST_CHECK_MS);
    const daily = setInterval(check, CHECK_EVERY_MS);
    return () => {
      clearTimeout(first);
      clearInterval(daily);
    };
  }, [enabled, isDevBuild]);
}
