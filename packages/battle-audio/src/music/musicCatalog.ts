// The sound catalog's `music` section: moods to tracks, each track a streamed
// file. Free of the rest of the catalog, so the generation tool can name a
// track's served file without loading the synthesised bank.

/** A streamed track, served as `music/<id>.<sha8>.mp3` (see `musicUrl`). */
export interface MusicTrack {
  /** The `sources` record it was encoded from. */
  source: string;
  /** Of the served MP3. */
  sha256: string;
  gain: number;
}
export const MOODS = ["menu", "tension", "combat"] as const;
export type Mood = (typeof MOODS)[number];
export interface MusicCatalog {
  moods: Record<Mood, string[]>;
  tracks: Record<string, MusicTrack>;
}
/** The served file: a content hash in the name, so a re-encoded track never meets a cached one. */
export const musicUrl = (id: string, track: MusicTrack) =>
  `/audio/music/${id}.${track.sha256.slice(0, 8)}.mp3`;
