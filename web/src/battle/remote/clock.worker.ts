// Each remote connection owns this worker and terminates it when it closes.
function runClock() {
  const waiting = new Map<number, ReturnType<typeof setTimeout>>();
  self.onmessage = ({ data }: MessageEvent<{ id: number; delayMs?: number }>) => {
    clearTimeout(waiting.get(data.id));
    waiting.delete(data.id);
    if (data.delayMs === undefined) return;
    waiting.set(
      data.id,
      setTimeout(() => {
        waiting.delete(data.id);
        self.postMessage(data.id);
      }, data.delayMs),
    );
  };
}
runClock();
export {};
