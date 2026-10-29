# 03 Spike: sound sources

**Question:** do real recordings exist for every firing sound the content pass
([15](15-content.md)) needs?

**Needed:**
- firing sounds: artillery, mortar, rocket salvo (rocket pods), bomb release,
  `artillery_strike_missile`;
- burst recordings at each weapon's cadence: marksman rifle, heavy sniper, the
  AA guns (cadence from each row's fire interval and burst; see
  `sound/frame.rs:846-866`).

**Seam:** none in code. A source ledger in this file.
- Each entry: URL, author, license, the crop in seconds, and why it fits.
- Candidates go in `throwaway/sound-sources/`, fetched with `yt-dlp` as the
  catalog already does (`packages/battle-audio/tools/assets.py`).
- The repository is private, so All-Rights-Reserved sources are acceptable
  (user, 2026-10-29).

**Run / hear:** one trimmed WAV per candidate, opened for the user. This is a
non-blocking checkpoint: about 5 minutes, then pick on the evidence.

**Verify:** each burst candidate's shot spacing matches its weapon's cadence
within 5 ms per shot.

**Moves:** nothing.

**Delegated:** source search and crop choice. A recipe layered from existing
clips stands in only where no source is found; record it as a fallback.

**Would change 15:** a missing source. Note the fallback here so 15 doesn't
discover it late.

## Ledger

Research receipt, 2026-10-29. All downloads, metadata, hashes, measurement
scripts and WAVs are outside the checkout at
`/Users/server/dev/battlesim/throwaway/sound-sources/`. `receipts.json` pins
the 18 audio downloads and their original crops; `auditions/` also contains
four shorter selections. Downloads used `yt-dlp 2026.08.19`; auditions are
mono 48 kHz, 16-bit PCM, without normalization, time stretching or added
layers. Crop times below are seconds in the original YouTube upload.

Rights below mean **All-Rights-Reserved**: none of these downloads declares
an open license. A military origin in a description alone is not treated as
a public-domain license for the uploader's edit. This is acceptable under
this spike's private-repository decision.

The source videos' frames and decoded waveforms were inspected. The 22 WAVs
were opened in QuickTime Player for the non-blocking listening checkpoint;
agent-side timbre, intelligible speech and music absence are **not verified**.
The choices are source/crop recommendations, not accepted runtime assets.
In particular, the rocket-pod recommendation needs listening to separate its
motor from the rotor bed, and no isolated bomb-release recording is accepted.

| Need | Chosen source, author, license, crop and reason | Runner-up | Cadence verdict / fallback for 15 |
|---|---|---|---|
| Artillery firing | [M777 direct fire](https://www.youtube.com/watch?v=KX6OPMngT2g), FUNKER530 / original camera operator uncredited; All-Rights-Reserved; **10.02–10.90 s**. Frames show the howitzer firing just after 10 s; this crop selects the muzzle event, before the camera turns toward the later impact. `auditions/artillery-KX6OPMngT2g.wav`. | [Marine Corps Artillery](https://www.youtube.com/watch?v=CdO_0twP_CU), TheBoon1989; All-Rights-Reserved; **87.50–89.50 s**, candidate strong report in a longer firing mission; not visually localized as precisely as the choice. | A real launch source exists. Shorten around the attack and fade its retained tail after listening; avoid commands and handling. No impact substituted for firing. |
| Mortar firing | [82 mm mortar firing and impact](https://www.youtube.com/watch?v=pujCGZMn_o4), Civ Div; All-Rights-Reserved; **2.10–3.20 s**. A distinct launch onset at approximately **2.169 s**; the impact is around 31 s and is excluded. `auditions/mortar-pujCGZMn_o4.wav`. | [Marines Fire 81mm Mortars](https://www.youtube.com/watch?v=ntRPvd6Sl0k), OkraJoe; footage credited to Cpl. Joseph F. Sanchez / USMC; All-Rights-Reserved for this upload; **8.55–9.45 s**. | Real mortar launch rather than a distant detonation; keep separate from the howitzer if the content pass needs the difference. |
| Rocket salvo, helicopter/aircraft pods | [AH-64 Apache: rocket launch and gun live fire](https://www.youtube.com/watch?v=IpPMbbuSOuA), AiirSource Military; All-Rights-Reserved; **33.00–39.00 s**. Frames show fresh rocket trails leaving the helicopter at approximately 33.5 s. This is a real pod-launch recording, with rotor noise and propagation delay; **conditional crop**, not a clean isolated motor. `auditions/rocket-pod-IpPMbbuSOuA.wav`. | [AH-1F rocket raid rehearsal](https://www.youtube.com/watch?v=72ICQlne6ms), Army News & Docs, described as DoD footage; All-Rights-Reserved for this upload; **12.00–16.00 s**. Rotor-heavy alternative. Also retained Apache candidates [Combat Films](https://www.youtube.com/watch?v=0cvLQqy73HY), **10.70–12.20 s**, and [Operator Close Air Support](https://www.youtube.com/watch?v=ntsyxUBUSK0), **1.45–3.50 s**; cannon/impact contamination prevents accepting them as isolated rocket motors. | If the motor cannot be separated at audition, **none found → recipe fallback**: existing `at4-launch` attack plus `missile-exhaust` tail, one launch per observed rocket. The current `rocket_pod` interval is 120 ms with an eight-round magazine and no authored burst group; do not play a second full salvo for every launch. |
| Bomb release and fall, excluding explosion | **none found → recipe fallback.** Best rejected evidence: [F-16 Cluster Drop](https://www.youtube.com/watch?v=ec4fiHL_wM4), We are the Air Force; All-Rights-Reserved for this upload; **0.05–3.50 s**. Frames show release/fall, but this does not establish an isolated audible ejector or falling-bomb sound. `auditions/bomb-release-ec4fiHL_wM4.wav`. | [F-15 Bomb Drop](https://www.youtube.com/watch?v=cRRMBITl_B8), We are the Air Force; All-Rights-Reserved; **10.70–13.50 s**, visible release with aircraft/background audio. [Northern Lightning](https://www.youtube.com/watch?v=BGnQD1cIEO0), War Footage Uncut; All-Rights-Reserved; **0.00–5.00 s** retained as a rejected preview; its nine-minute upload was downloaded, but the full upload was not exhaustively reviewed. | Use an attenuated short `reload-13` mechanical fragment for the ejector and a filtered non-impact tail of `ricochet-1` for the falling-air gesture. Keep it restrained and unpowered: no explosion or missile ignition at release. This is a proposed design from existing clips, not a real bomb recording or a rendered/approved recipe. |
| `artillery_strike_missile` launch | [ATACMS / Hyunmoo exercise](https://www.youtube.com/watch?v=8y_1z35LQ40), AiirSource Military; footage credited to DoD / Sinthia Rosario; All-Rights-Reserved for the derivative upload; **9.40–13.00 s**. M270-launched ATACMS event: sharp onset around **9.472 s**, followed by sustained launch energy. Crop excludes target impact. `auditions/strike-missile-8y_1z35LQ40.wav`. | **20.03–23.50 s** in the same upload, a second M270 launch with onset around **20.103 s**; `auditions/strike-missile-runner-up.wav`. This is another take, not an independent source. | Real large ground strike-missile launch, rather than a shoulder launcher or aircraft pod. One report: this weapon has no magazine/burst and a 15 s reload. |
| Marksman rifle | [Firing the M110 SASS](https://www.youtube.com/watch?v=Jkt3S5WpJN8), abnhawk; All-Rights-Reserved; **3.66–4.60 s** for a single report (`auditions/marksman-single.wav`), **3.60–16.50 s** comparison crop. Principal attacks around **3.682, 9.491, 15.533 s** give **5,809 and 6,042 ms**, versus the row's **500 ms** shot interval: fails the 5 ms burst requirement. | [M110 live-fire range, Eagle Wrath 22](https://www.youtube.com/watch?v=E3YVJHpoa9c), Devil Dog TV; film credited to Cpl. Evan Jones; All-Rights-Reserved for this upload; **10.80–12.50 s**. Mixed transient/handling evidence; no reliable isolated 500 ms sequence established. | Matching natural burst: **none found → recipe fallback**. The resolved row explicitly sets `magazine.burst: null`; use one recorded report per observed launch, not an inherited three-shot rifle clip. If a future authored burst requires a constructed sequence, the existing `assets.py` relay can place distinct retained reports exactly 500 ms apart; that is a designed fallback, not a matching raw recording. |
| Heavy sniper | [BARRETT 50 CAL RAPID FIRE](https://www.youtube.com/watch?v=UFB7-bvVKs8), K / @kenny2887; All-Rights-Reserved; **9.18–10.00 s** selects its last report and tail (`auditions/heavy-sniper-single.wav`); **6.95–9.80 s** retains the comparison burst. Consecutive clear attacks **7.244, 7.476, 7.706, 7.935 s** give **232, 230, 229 ms**. | [Barrett M82A1 shoulder rapid fire](https://www.youtube.com/watch?v=mSKckg0MiMI), ms500hp1; All-Rights-Reserved; **15.60–18.80 s**. Dense, contaminated rapid reports; automated prominent-transient gaps span **145–298 ms**, not a verified slow-fire sequence. | A heavy-rifle single report exists. A burst matching the game's cycle: **none found → recipe fallback**. The resolved row has **`magazine: null`**, hence no shot interval or burst; **2.5 s is `reload_s`, not a burst interval**. Use a single recorded report per launch. Do not claim the 229–232 ms Barrett sequence matches, or construct a 2.5 s burst by misreading reload. |
| AA guns (`aa_ballistic`; `autocannon` has the same authored cadence) | [Bofors E/L 70 Shoot](https://www.youtube.com/watch?v=Zj4xZlU-1wM), IrishGunner; All-Rights-Reserved; **19.65–20.35 s** (`auditions/aa-three-reports.wav`). Clear attack edges **19.687, 19.888, 20.107 s** give **201 and 219 ms**, versus **300 ms × three rounds**: errors **99 and 81 ms**, fails. | [Second Bofors E/L 70 take](https://www.youtube.com/watch?v=E5P5_9v8Oaw), IrishGunner; All-Rights-Reserved; **5.45–6.30 s**. Prominent attack spacings **207 and 214 ms** (echo-rich; onset classification less certain); also fails 300 ± 5 ms. | **none found → recipe fallback** for an unchanged natural three-round recording. Use the existing relay to cut three separate reports before their next source attack and place them at **0, 0.300, 0.600 s**. If restricted to already catalogued clips, layer three existing `cannon-m1` crops with restrained gain/pitch instead; the retimed Bofors source is the better research candidate but remains a new asset to admit in 15. |

Measurements use decoded source time, 1 ms RMS bins and inspection of the
attack edge; echoes, handling and speech are not counted as additional shots
in the principal sequences above. Automated candidates and their less certain
classifications remain in `measurements.json`; the manually selected principal
edges above take precedence over its generic peak detector. The observed
errors are much larger than the 5 ms gate. No searched burst is labelled a
pass, and no resampled/relayed output is presented as an original recording.

The cadence reader in `sound/frame.rs` suppresses launches already covered by
a recording; its half-interval tolerance is not this spike's 5 ms acceptance
criterion. Explicit `null` in `marksman_rifle` and `heavy_sniper` replaces the
inherited rifle burst/magazine. Their single-report recommendation preserves
those rules. No fixture, runtime audio or battle outcome changed.
