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
    "handling": "highpass=f=60",
    "launch": "highpass=f=40,acompressor=threshold=0.18:ratio=2.2:attack=5:release=90",
    "motor-loop": "highpass=f=80,lowpass=f=6000",
    "blast": "highpass=f=30,acompressor=threshold=0.18:ratio=2.2:attack=5:release=90",
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


def relay(source, burst, rate, start, end):
    """Overlap-add each source shot `interval_s` after the last, at the gun's cadence.

    A shot is cut where its retained tail ends, so a faster recording cannot
    carry its next attack into this one; the cut fades over 10 ms."""
    step = round(burst["interval_s"] * rate)
    shots = burst["shots"]
    if len(shots) < 2 or step <= 0 or any(not (start <= a < b <= end) for a, b in shots):
        raise ValueError("invalid burst shots or interval")
    out = array.array("f", bytes(4 * max(k * step + b - a for k, (a, b) in enumerate(shots))))
    for k, (a, b) in enumerate(shots):
        shot = source[a:b]
        fade = min(480, len(shot) // 4)
        for i in range(fade):
            shot[-1 - i] *= i / fade
        for i, value in enumerate(shot):
            out[k * step + i] += value
    return out


def render(clip, decoded):
    rate, source = decoded[clip["source"]]
    start, end = clip["source_frames"]
    if rate != clip["source_rate"] or not (0 <= start < end <= len(source)):
        raise ValueError("invalid source frame range/rate")
    profile = clip["processing"]
    if profile not in FILTERS:
        raise ValueError(f"unknown processing profile {profile}")
    burst = clip.get("burst")
    crop = relay(source, burst, rate, start, end) if burst else source[start:end]
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["build", "check", "list"])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    args = parser.parse_args()
    path = args.root / "fixtures/sounds.json"
    catalog = json.loads(path.read_text())
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
