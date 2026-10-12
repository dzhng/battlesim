#!/usr/bin/env python3
"""Rebuild/check the pinned audio library. Needs Python 3 and ffmpeg on PATH."""
import argparse
import array
import hashlib
import io
import json
from pathlib import Path
import struct
import subprocess
import sys
import wave

FILTERS = {
    "weighted-shot": "highpass=f=45,equalizer=f=140:t=o:w=1.2:g=4,equalizer=f=3000:t=o:w=1:g=1.5,acompressor=threshold=0.18:ratio=2.2:attack=5:release=90",
    "heavy-report": "highpass=f=35,equalizer=f=100:t=o:w=1.2:g=4,acompressor=threshold=0.18:ratio=2.2:attack=5:release=90",
    "impact": "highpass=f=60",
    "heavy-impact": "highpass=f=35,lowpass=f=4500,equalizer=f=120:t=o:w=1:g=3",
    "armour-impact": "highpass=f=30",
    "handling": "highpass=f=60",
    "launch": "highpass=f=40,acompressor=threshold=0.18:ratio=2.2:attack=5:release=90",
    "motor-loop": "highpass=f=80,lowpass=f=6000",
    "running-gear": "highpass=f=50,lowpass=f=5000",
    "blast": "highpass=f=30,acompressor=threshold=0.18:ratio=2.2:attack=5:release=90",
}

# Generated voice takes: where the speaker sits decides the treatment (choices.md T2).
# A hiss is noise band-limited like the radio. A bed is the machine's ambience, looped
# under the line through its own filter: the game's own sound for that machine (`clip`,
# a sounds.json clip), or a generated raw master (`generated`, a voices.lock.json bed)
# where the game has no recording. Each take is normalised to its profile's loudness.
VOICE_ENCODER = {"codec": "libmp3lame", "bitrate": "40k", "sample_rate": 24000, "channels": 1,
                 "true_peak_db": -1.5}
VOICE_PROFILES = {
    "voice-dry": {"filter": "anull", "hiss": None, "bed": None, "loudness_lufs": -16},
    "voice-radio-light": {
        "filter": "highpass=f=250,lowpass=f=3800,acompressor=threshold=0.1:ratio=4:attack=5:release=60:makeup=2",
        "hiss": {"color": "white", "amplitude": 0.015, "highpass": 500, "lowpass": 3000, "weight": 0.3},
        # The game synthesises its engines; only the track rattle is recorded.
        "bed": {"generated": "tank_engine", "filter": "anull", "volume": 0.3},
        "loudness_lufs": -16,
    },
    "voice-earpiece": {
        "filter": "highpass=f=250,lowpass=f=4500,acompressor=threshold=0.1:ratio=4:makeup=2",
        "hiss": {"color": "pink", "amplitude": 0.01, "highpass": None, "lowpass": 3000, "weight": 0.5},
        "bed": None,
        "loudness_lufs": -19,
    },
    "voice-intercom": {
        "filter": "highpass=f=300,lowpass=f=3400,acompressor=threshold=0.1:ratio=6:attack=5:release=60:makeup=3",
        "hiss": {"color": "white", "amplitude": 0.015, "highpass": 500, "lowpass": 3000, "weight": 0.25},
        # Cockpit-muffled: the rotor heard through the airframe and the headset.
        "bed": {"clip": "rotor-uh60", "filter": "lowpass=f=900", "volume": 0.6},
        "loudness_lufs": -16,
    },
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def floats(data):
    result = array.array("f")
    result.frombytes(data)
    if sys.byteorder != "little":
        result.byteswap()
    return result


def ffmpeg(arguments, data=None):
    result = subprocess.run(["ffmpeg", "-v", "error", *arguments], input=data,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        raise ValueError(result.stderr.decode().strip())
    return result.stdout


def sources(root, catalog):
    decoded = {}
    # Admit every source before writes so a changed master cannot partly replace a library.
    for name, source in catalog["sources"].items():
        path = root / source["path"]
        if not path.is_file() or digest(path.read_bytes()) != source["sha256"]:
            raise ValueError(f"source {name}: hash mismatch or missing media (pull Git LFS)")
    for name, source in catalog["sources"].items():
        path = root / source["path"]
        info = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-show_streams", "-of", "json", str(path)]))["streams"][0]
        rate = int(info["sample_rate"])
        channels = int(info["channels"])
        if channels not in (1, 2):
            raise ValueError(f"source {name}: expected mono or stereo")
        args = ["-i", str(path)]
        if channels == 2:
            args += ["-af", "pan=mono|c0=0.5*c0+0.5*c1"]
        decoded[name] = (rate, floats(ffmpeg([*args, "-f", "f32le", "-"])))
    return decoded


def overlay(parts):
    """Sum (offset, samples) parts into one buffer."""
    out = array.array("f", bytes(4 * max(at + len(x) for at, x in parts)))
    for at, x in parts:
        for i, value in enumerate(x):
            out[at + i] += value
    return out


def cut(decoded, name, rate, frames):
    source_rate, source = decoded[name]
    start, end = frames
    if rate != source_rate or not (0 <= start < end <= len(source)):
        raise ValueError("invalid source frame range/rate")
    return source[start:end]


def relay(source, burst, rate, start, end):
    """Overlap-add each source shot `interval_s` after the last, at the gun's cadence.

    A shot is cut where its retained tail ends, so a faster recording cannot
    carry its next attack into this one; the cut fades over 10 ms."""
    step = round(burst["interval_s"] * rate)
    shots = burst["shots"]
    if len(shots) < 2 or step <= 0 or any(not (start <= a < b <= end) for a, b in shots):
        raise ValueError("invalid burst shots or interval")
    parts = []
    for k, (a, b) in enumerate(shots):
        shot = source[a:b]
        fade = min(480, len(shot) // 4)
        for i in range(fade):
            shot[-1 - i] *= i / fade
        parts.append((k * step, shot))
    return overlay(parts)


def layer(decoded, spec):
    """One layer at 48 kHz: its crop pitched by resampling (lower is also slower,
    as a bigger, slower body would sound), optionally low-passed, at its gain,
    fading out over its last quarter, at most 150 ms."""
    crop = cut(decoded, spec["source"], spec["source_rate"], spec["source_frames"])
    if sys.byteorder != "little":
        crop.byteswap()
    shifted = round(spec["source_rate"] * 2 ** (spec["semitones"] / 12))
    filters = [f"asetrate={shifted}", "aresample=48000"]
    if spec["lowpass_hz"]:
        filters += [f"lowpass=f={spec['lowpass_hz']}"] * 2
    x = floats(ffmpeg(["-f", "f32le", "-ar", str(spec["source_rate"]), "-ac", "1", "-i", "-",
                       "-af", ",".join(filters), "-f", "f32le", "-"], crop.tobytes()))
    # A crop may end mid-rumble; fade it out rather than cut it off.
    fade = min(len(x) // 4, 7200)
    for i in range(fade):
        x[-1 - i] *= i / fade
    return array.array("f", (v * spec["gain"] for v in x))


def render(clip, decoded):
    profile = clip["processing"]
    if profile not in FILTERS:
        raise ValueError(f"unknown processing profile {profile}")
    if "layers" in clip:
        if not clip["layers"] or any(spec["at_s"] < 0 for spec in clip["layers"]):
            raise ValueError("invalid layers")
        rate = 48000
        crop = overlay([(round(spec["at_s"] * rate), layer(decoded, spec)) for spec in clip["layers"]])
    else:
        rate = clip["source_rate"]
        start, end = clip["source_frames"]
        source = decoded[clip["source"]][1]
        crop = cut(decoded, clip["source"], rate, clip["source_frames"])
        burst = clip.get("burst")
        if burst:
            crop = relay(source, burst, rate, start, end)
    if sys.byteorder != "little":
        crop.byteswap()
    x = floats(ffmpeg(["-f", "f32le", "-ar", str(rate), "-ac", "1", "-i", "-",
                       "-af", FILTERS[profile], "-ar", "48000", "-f", "f32le", "-"], crop.tobytes()))
    if not x or any(not (-10 < value < 10) for value in x):
        raise ValueError("empty or invalid PCM")
    if clip["loop"]:
        cross = min(4800, len(x) // 8)
        result = x[cross:]
        for i in range(cross):
            t = i / cross
            result[len(result) - cross + i] = x[len(x) - cross + i] * (1 - t) + x[i] * t
        x = result
    else:
        fade = min(240, len(x) // 8)
        for i in range(fade):
            x[-1-i] *= i / fade
        for i in range(min(12, len(x))):
            x[i] *= i / 12
    peak = max(abs(v) for v in x)
    if peak < 0.00001:
        raise ValueError("silent clip")
    scale = 0.72 / peak
    pcm = b"".join(struct.pack("<h", round(v * scale * 32767)) for v in x)
    output = io.BytesIO()
    with wave.open(output, "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(48000)
        wav.writeframes(pcm)
    return output.getvalue(), len(x)


def loudness(pcm, rate):
    """Integrated loudness (LUFS, EBU R128) of mono f32 PCM."""
    result = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-f", "f32le", "-ar", str(rate), "-ac", "1",
                             "-i", "-", "-af", "ebur128", "-f", "null", "-"],
                            input=pcm, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    summary = result.stderr.decode().rpartition("Summary:")[2]
    for line in summary.splitlines():
        if line.strip().startswith("I:"):
            return float(line.split()[1])
    raise ValueError(result.stderr.decode().strip() or "no loudness measured")


def voice(profile, source, bed, output):
    """Render one generated take through its profile to mono MP3 at the profile's loudness.

    The bed starts at an offset taken from the take's bytes, so neighbouring lines
    don't open on the same engine note, and the hiss is seeded the same way: a
    take always renders alike on one ffmpeg build."""
    spec = VOICE_PROFILES.get(profile)
    if spec is None:
        raise ValueError(f"unknown voice profile {profile}")
    if bool(spec["bed"]) != bool(bed):
        raise ValueError(f"profile {profile}: " + ("needs a bed (--bed)" if spec["bed"] else "takes no bed"))
    seed = int(digest(source.read_bytes())[:8], 16)
    rate = 48000
    args = ["-i", str(source)]
    graph = [f"[0:a]aformat=channel_layouts=mono,aresample={rate},{spec['filter']}[voice]"]
    mix, weights = ["[voice]"], ["1"]
    hiss = spec["hiss"]
    if hiss:
        bands = "".join(f",{kind}=f={hiss[kind]}" for kind in ("highpass", "lowpass") if hiss[kind])
        graph.append(f"anoisesrc=color={hiss['color']}:amplitude={hiss['amplitude']}:sample_rate={rate}"
                     f":seed={seed % 2**31}{bands}[hiss]")
        mix.append("[hiss]")
        weights.append(str(hiss["weight"]))
    if bed:
        length = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration",
                                                "-of", "csv=p=0", str(bed)]))
        args += ["-stream_loop", "-1", "-ss", f"{(seed % 1000) / 1000 * length * 0.9:.3f}", "-i", str(bed)]
        graph.append(f"[1:a]aformat=channel_layouts=mono,aresample={rate},"
                     f"{spec['bed']['filter']},volume={spec['bed']['volume']}[bed]")
        mix.append("[bed]")
        weights.append("1")
    graph.append(f"{''.join(mix)}amix=inputs={len(mix)}:duration=first:normalize=0:weights={' '.join(weights)}[out]")
    pcm = ffmpeg([*args, "-filter_complex", ";".join(graph), "-map", "[out]", "-ac", "1", "-ar", str(rate),
                  "-f", "f32le", "-"])
    frames = len(pcm) // 4
    if frames < rate // 2:
        raise ValueError("take shorter than half a second")
    measured = loudness(pcm, rate)
    if measured < -60:
        raise ValueError("silent take")
    duration = frames / rate
    # Linear gain to the target keeps the line's dynamics; the limiter only catches peaks.
    limit = 10 ** (VOICE_ENCODER["true_peak_db"] / 20)
    chain = (f"volume={spec['loudness_lufs'] - measured:.2f}dB,alimiter=limit={limit:.4f}:level=0,"
             f"afade=t=in:d=0.005,afade=t=out:st={max(0, duration - 0.05):.3f}:d=0.05")
    output.write_bytes(ffmpeg(["-f", "f32le", "-ar", str(rate), "-ac", "1", "-i", "-", "-af", chain,
                               "-ac", str(VOICE_ENCODER["channels"]), "-ar", str(VOICE_ENCODER["sample_rate"]),
                               "-c:a", VOICE_ENCODER["codec"], "-b:a", VOICE_ENCODER["bitrate"],
                               "-map_metadata", "-1", "-fflags", "+bitexact", "-flags:a", "+bitexact",
                               "-f", "mp3", "-"], pcm))
    return {"duration_s": round(duration, 3), "loudness_lufs": spec["loudness_lufs"]}

def music(root, catalog, path):
    """Encode each music track from its pinned source to runtime/audio/music/<track>.<sha8>.mp3."""
    for name, track in catalog["music"]["tracks"].items():
        source = catalog["sources"][track["source"]]
        master = root / source["path"]
        if not master.is_file() or digest(master.read_bytes()) != source["sha256"]:
            raise ValueError(f"music {name}: source hash mismatch or missing media (pull Git LFS)")
        # Mono like the rest of the library; 128 kbps streams without a whole-file decode.
        # Encoded to a file, not a pipe, so ffmpeg can write the seekable header.
        folder = root / "assets/runtime/audio/music"
        folder.mkdir(parents=True, exist_ok=True)
        encoded = folder / f"{name}.tmp.mp3"
        ffmpeg(["-y", "-i", str(master), "-ac", "1", "-b:a", "128k", str(encoded)])
        served = digest(encoded.read_bytes())
        target = folder / f"{name}.{served[:8]}.mp3"
        encoded.replace(target)
        if track["sha256"] != served:
            (folder / f"{name}.{track['sha256'][:8]}.mp3").unlink(missing_ok=True)
            # The hash is unique in the file: replace it in place, keeping the catalog's layout.
            path.write_text(path.read_text().replace(track["sha256"], served))
        print(f"music: {target.relative_to(root)}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["build", "check", "list", "voice", "voice-profiles", "music"])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--profile", help="voice: the take's processing profile")
    parser.add_argument("--input", type=Path, help="voice: the raw master")
    parser.add_argument("--bed", type=Path, help="voice: the profile's ambience, looped under the line")
    parser.add_argument("--output", type=Path, help="voice: the served MP3")
    args = parser.parse_args()
    if args.command == "voice-profiles":
        print(json.dumps({"encoder": VOICE_ENCODER, "profiles": VOICE_PROFILES}))
        return
    if args.command == "voice":
        if not (args.profile and args.input and args.output):
            raise ValueError("voice needs --profile, --input and --output")
        print(json.dumps(voice(args.profile, args.input, args.bed, args.output)))
        return
    path = args.root / "fixtures/sounds.json"
    catalog = json.loads(path.read_text())
    if args.command == "music":
        music(args.root, catalog, path)
        return
    if args.command == "list":
        for name, clip in catalog["clips"].items():
            print(f"{name}: {clip['label']} ({clip['role']})")
        return
    decoded = sources(args.root, catalog)
    prepared = {}
    for name, clip in catalog["clips"].items():
        if clip["url"] != f"/audio/clips/{name}.wav" or not name.replace("-", "").replace("_", "").isalnum():
            raise ValueError(f"clip {name}: invalid runtime identity")
        try:
            prepared[name] = render(clip, decoded)
        except (ValueError, KeyError) as error:
            raise ValueError(f"clip {name}: {error}") from error
    for name, (data, frames) in prepared.items():
        clip = catalog["clips"][name]
        target = args.root / "assets/runtime" / clip["url"].lstrip("/")
        if args.command == "check":
            if not target.is_file() or target.read_bytes() != data or clip["sha256"] != digest(data) or clip["frames"] != frames or clip["sample_rate"] != 48000:
                raise ValueError(f"clip {name}: prepared bytes/metadata differ; rebuild or pull Git LFS")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            clip.update(sha256=digest(data), frames=frames, sample_rate=48000)
    if args.command == "build":
        path.write_text(json.dumps(catalog, indent=2) + "\n")
    print(f"{args.command}: {len(prepared)} clips, {len(decoded)} pinned sources")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, FileNotFoundError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
