import {
  createContext,
  Suspense,
  useCallback,
  useContext,
  useEffect,
  useId,
  useState,
  type ReactNode,
} from "react";
import { LoadingScreen } from "./LoadingScreen";

type Stage = "preparation" | "world" | "renderer";
interface Task {
  stage: Stage;
  ready: boolean;
  error?: string | null;
}
const Progress = createContext<((id: string, task: Task | null) => void) | null>(null);
export const LOADING_STAGES: readonly { id: Stage; label: string }[] = [
  { id: "preparation", label: "Preparing the battle" },
  { id: "world", label: "Building the battlefield" },
  { id: "renderer", label: "Starting the battle" },
];

/** How far the tasks reported inside one loading boundary have come. */
export interface LoadingProgress {
  /** The stage in progress; the last one once everything is ready. */
  current: Stage;
  /** Every reported task is ready, and a viewport has reported in. */
  done: boolean;
  error: string | null;
}

/** The tasks reported by `useLabLoading` under one `LoadingTasks` boundary.
 *  Preparation and simulation can finish independently of the first frame. */
export function useLoadingTasks() {
  const [error, setError] = useState<string | null>(null);
  const [tasks, setTasks] = useState<Record<string, Task>>({});
  const report = useCallback((id: string, task: Task | null) => {
    if (task?.error) setError(task.error);
    setTasks((previous) => {
      const next = { ...previous };
      if (task) next[id] = task;
      else delete next[id];
      return next;
    });
  }, []);
  const values = Object.values(tasks);
  const pending = LOADING_STAGES.find((stage) =>
    values.some((task) => task.stage === stage.id && !task.ready),
  );
  // Until a viewport (or a replay import view) reports in, preparation is
  // still mounting. A route returning null must not leave a blank page.
  const waiting = !values.some((task) => task.stage === "renderer");
  const progress: LoadingProgress = {
    current: pending?.id ?? (waiting ? (values.length ? "world" : "preparation") : "renderer"),
    done: !waiting && !pending,
    error,
  };
  return { report, progress };
}

/** The boundary `useLabLoading` reports to, with `report` from `useLoadingTasks`. */
export function LoadingTasks({
  report,
  children,
}: {
  report: ReturnType<typeof useLoadingTasks>["report"];
  children: ReactNode;
}) {
  return <Progress value={report}>{children}</Progress>;
}

/** Covers every lab, including routes that compose the viewport directly. */
export function LabLoading({ subject, children }: { subject: string; children: ReactNode }) {
  const { report, progress } = useLoadingTasks();
  const { error } = progress;
  const cover = (
    <LoadingScreen
      title="Deploying"
      subject={subject}
      stages={LOADING_STAGES}
      current={progress.current}
      failure={error ? { message: "The battle could not be prepared.", details: [error] } : null}
      back="/labs"
    />
  );
  return (
    <LoadingTasks report={report}>
      {/* A refusal owns the page and unmounts failed work, preserving one error surface. */}
      {!error && <Suspense fallback={null}>{children}</Suspense>}
      {(!progress.done || error) && cover}
    </LoadingTasks>
  );
}

/** Report readiness; null withdraws a task when a route shows a different view.
 *  No effect outside a lab loading boundary. */
export function useLabLoading(stage: Stage, ready: boolean | null, error?: string | null) {
  const report = useContext(Progress);
  const id = useId();
  useEffect(() => {
    report?.(id, ready === null ? null : { stage, ready, error });
    return () => report?.(id, null);
  }, [report, id, stage, ready, error]);
}
