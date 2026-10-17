/** Records one element of this tab, with the tab's sound, as a VP9+Opus WebM.
 *
 *  The browser composites the stage exactly as the player sees it (WebGPU
 *  canvases, DOM cards, CSS grade) and mixes every sound the tab plays, so
 *  nothing here re-renders or re-mixes. Chromium grants the capture without a
 *  prompt only when launched with `--auto-accept-this-tab-capture`; without it
 *  `getDisplayMedia` waits on a dialog nobody will answer, so the caller owns
 *  a timeout. */

/** Region Capture: not in TypeScript's DOM lib yet. */
interface CropTarget {}
interface CropTargetConstructor {
  fromElement(element: Element): Promise<CropTarget>;
}
type CroppableTrack = MediaStreamTrack & { cropTo?(target: CropTarget): Promise<void> };

/** The Screen Capture options Chromium reads that TypeScript's DOM lib lacks. */
interface TabCaptureOptions extends DisplayMediaStreamOptions {
  /** Offer this tab first (and, with the auto-accept flag, take it). */
  preferCurrentTab?: boolean;
  selfBrowserSurface?: "include" | "exclude";
  audio?: boolean | (MediaTrackConstraints & { suppressLocalAudioPlayback?: boolean });
}

/** Why a recording could not start. Each is a setup fault, never a retry. */
export type TabRecorderFault =
  | "no-video-track"
  | "no-audio-track"
  | "crop-unsupported"
  | "codec-unsupported";

export class TabRecorderError extends Error {
  override name = "TabRecorderError";
  constructor(
    readonly fault: TabRecorderFault,
    message: string,
  ) {
    super(`${fault}: ${message}`);
  }
}

export interface TabRecording {
  /** Video frames the capture delivered while recording. */
  frames: number;
  /** Longest wait between two delivered frames. */
  worstGapMs: number;
  /** Wall-clock length of the recording. */
  seconds: number;
}

export interface RecordTabOptions {
  /** The frame rate asked of the capture; the page may draw fewer. */
  fps: number;
  /** Recording stops when this settles, whether it resolves or rejects. */
  until: Promise<void>;
  /** The recorder is running: from here on, every frame is recorded. */
  onStart?: () => void;
  /** Each piece of the WebM, in order. The next waits for this one's
   *  promise, and the recording ends only once the last has settled. */
  onChunk: (chunk: Blob) => Promise<void> | void;
}

const MIME_TYPE = "video/webm;codecs=vp9,opus";
const VIDEO_BITS_PER_SECOND = 20_000_000;
const AUDIO_BITS_PER_SECOND = 192_000;
const CHUNK_MS = 1000;

/** The tab's mix as it plays: stereo, with none of the voice-call processing
 *  a microphone gets, and still heard in the tab while it records. */
const TAB_AUDIO = {
  channelCount: 2,
  echoCancellation: false,
  noiseSuppression: false,
  autoGainControl: false,
  suppressLocalAudioPlayback: false,
};

/** Record `stage` until `until` settles, streaming the WebM to `onChunk`.
 *  The capture stops with it, whether it resolves or rejects. */
export async function recordTab(
  stage: HTMLElement,
  { fps, until, onStart, onChunk }: RecordTabOptions,
): Promise<TabRecording> {
  if (!MediaRecorder.isTypeSupported(MIME_TYPE))
    throw new TabRecorderError("codec-unsupported", `MediaRecorder cannot write ${MIME_TYPE}`);
  const cropTarget = (globalThis as { CropTarget?: CropTargetConstructor }).CropTarget;
  if (!cropTarget)
    throw new TabRecorderError(
      "crop-unsupported",
      "this browser has no CropTarget (Region Capture)",
    );

  const options: TabCaptureOptions = {
    video: { frameRate: fps },
    audio: TAB_AUDIO,
    preferCurrentTab: true,
    selfBrowserSurface: "include",
  };
  const stream = await navigator.mediaDevices.getDisplayMedia(options);
  try {
    const [track] = stream.getVideoTracks() as CroppableTrack[];
    if (!track) throw new TabRecorderError("no-video-track", "the tab capture has no video track");
    if (stream.getAudioTracks().length === 0)
      throw new TabRecorderError(
        "no-audio-track",
        "the tab capture has no audio track; the browser did not share tab audio",
      );
    if (!track.cropTo)
      throw new TabRecorderError("crop-unsupported", "the capture track has no cropTo");
    await track.cropTo(await cropTarget.fromElement(stage));

    const counter = await countFrames(track);
    const recorder = new MediaRecorder(stream, {
      mimeType: MIME_TYPE,
      videoBitsPerSecond: VIDEO_BITS_PER_SECOND,
      audioBitsPerSecond: AUDIO_BITS_PER_SECOND,
    });
    let sent: Promise<void> = Promise.resolve();
    recorder.ondataavailable = (event) => {
      const chunk = event.data;
      if (chunk.size > 0) sent = sent.then(() => onChunk(chunk));
    };
    const stopped = new Promise<void>((resolve) => (recorder.onstop = () => resolve()));
    recorder.start(CHUNK_MS);
    const started = performance.now();
    counter.reset();
    onStart?.();
    try {
      await until;
    } finally {
      recorder.stop();
      await stopped;
      counter.stop();
    }
    const seconds = (performance.now() - started) / 1000;
    // The last chunk arrives before `stop`; its delivery may still be on its way.
    await sent;
    return { ...counter.result(), seconds };
  } finally {
    for (const track of stream.getTracks()) track.stop();
  }
}

/** Counts the frames the capture delivers, by playing the track into a
 *  detached video element and listening for each presented frame. */
async function countFrames(track: MediaStreamTrack) {
  const video = document.createElement("video");
  video.muted = true;
  video.srcObject = new MediaStream([track]);
  await video.play();
  let frames = 0;
  let worstGapMs = 0;
  let last: number | null = null;
  let presented: number | null = null;
  let handle = 0;
  const onFrame: VideoFrameRequestCallback = (now, metadata) => {
    // presentedFrames counts frames the element skipped between callbacks.
    frames += presented === null ? 1 : metadata.presentedFrames - presented;
    presented = metadata.presentedFrames;
    const at = metadata.captureTime ?? now;
    if (last !== null) worstGapMs = Math.max(worstGapMs, at - last);
    last = at;
    handle = video.requestVideoFrameCallback(onFrame);
  };
  handle = video.requestVideoFrameCallback(onFrame);
  return {
    reset() {
      frames = 0;
      worstGapMs = 0;
      last = null;
      presented = null;
    },
    stop() {
      video.cancelVideoFrameCallback(handle);
      video.srcObject = null;
    },
    result: () => ({ frames, worstGapMs }),
  };
}
