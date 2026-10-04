import { useEffect, useSyncExternalStore, type ReactNode } from "react";
import { appResources } from "./appResources";
import { LoadingScreen } from "./LoadingScreen";

export function AppResourceBoundary({ menu, children }: { menu: boolean; children: ReactNode }) {
  const error = useSyncExternalStore(appResources.subscribe, appResources.error);
  useEffect(() => {
    const leave = () => appResources.dispose();
    window.addEventListener("pagehide", leave);
    return () => {
      window.removeEventListener("pagehide", leave);
    };
  }, []);
  useEffect(() => {
    if (!menu || error) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const paint = requestAnimationFrame(() => {
      timer = setTimeout(() => void appResources.warm(), 0);
    });
    return () => {
      cancelAnimationFrame(paint);
      clearTimeout(timer);
    };
  }, [menu, error]);
  if (menu || !error) return children;
  return (
    <LoadingScreen
      title="Deploying"
      subject="Battle resources"
      stages={[]}
      current=""
      failure={{
        message: "The battle cannot continue.",
        advice: "Return to the menu or reload the page to prepare fresh resources.",
        details: [error],
      }}
      recovery={
        <button className="hud-menu-item" onClick={() => window.location.reload()}>
          Reload
        </button>
      }
    />
  );
}
