# Village parity review

**Target:** keep the historical village road placement, edge feather, grass exclusion and field cuts through the shared surface export. This is not an artwork or generated-city gate.

Both camera tours used the same route, fixture, seed, tick, cameras and 1920×1080 size. The original road, grass and camera assertions passed unedited; road telemetry stayed centre 5.5, inside [6,6], outside [38.5,37]. Paired metrics are diagnostic only: worst distance 0.00026 (pixelmatch 0.00015), road edge 0.00001 (pixelmatch 0). Strategic and isolated overlay shots are pixel-identical; the small deltas sit in grass detail.

No fresh critic could be started, so the review used the screenshot-critique adversarial fallback: argue the strongest break, then inspect full frames and enlarged crops.

| Strongest break case | Finding |
| --- | --- |
| Road cap shifted or gained an angular contour | Round contour, lower cut and grass match the paired crop. |
| Feather band or grass leak draws a false line at road edges | Both edges keep the same narrow blend and clear paved interior. |
| Plot guides split a field or moved a far road | Field layout and road runs match; strategic pixels unchanged. |
| Grass hides units or moves label attachment | Same grass, silhouettes and callouts; weak infantry contrast is an existing limitation. |
| Fog erases a junction or darkens pavement into a line | Junction geometry and fog boundary match. |

**Verdict:** village parity accepted for the structural surface contract. Flat tan paving, terrain and infantry readability stay open for the visual owner; mixed joins stay C63 work.
