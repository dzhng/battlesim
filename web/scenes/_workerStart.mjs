/** Arm one real worker start to remain paused at tick zero. */
export function installWorkerStartFence() {
  let pauseNextStart = false;
  const post = Worker.prototype.postMessage;
  Worker.prototype.postMessage = function (message, ...args) {
    if (pauseNextStart && message.type === "start") {
      pauseNextStart = false;
      post.call(this, { type: "pause" });
    }
    return post.call(this, message, ...args);
  };
  window.__pauseNextWorkerStart = () => {
    pauseNextStart = true;
  };
}
