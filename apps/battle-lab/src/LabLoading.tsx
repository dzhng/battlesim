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
const STAGES = [
  { id: "preparation", label: "Preparing the battle" },
  { id: "world", label: "Building the battlefield" },
  { id: "renderer", label: "Starting the battle" },
];

/** Covers every lab, including routes that compose the viewport directly.
 *  Preparation and simulation can finish independently of the first frame. */
export function LabLoading({ subject, children }: { subject: string; children: ReactNode }) {
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
  const pending = STAGES.find((stage) =>
    values.some((task) => task.stage === stage.id && !task.ready),
  );
  // Until a viewport (or a replay import view) reports in, preparation is
  // still mounting. A route returning null must not leave a blank page.
  const waiting = !values.some((task) => task.stage === "renderer");
  const cover = (
    <LoadingScreen
      title="Deploying"
      subject={subject}
      stages={STAGES}
      current={pending?.id ?? (values.length ? "world" : "preparation")}
      failure={error ? { message: "The battle could not be prepared.", details: [error] } : null}
      back="/labs"
    />
  );
  return (
    <Progress value={report}>
      {/* A refusal owns the page and unmounts failed work, preserving one error surface. */}
      {!error && <Suspense fallback={null}>{children}</Suspense>}
      {(waiting || pending || error) && cover}
    </Progress>
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
