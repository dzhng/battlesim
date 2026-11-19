// The menu's Settings page: graphics, sound and diagnostics.
import { DiagnosticsControls } from "../DiagnosticsControls";
import { GraphicsSettingsControls } from "../GraphicsSettingsControls";
import { SoundControls } from "../SoundControls";
import { runAutoDetect } from "../graphicsDetection";

import nh90Credits from "../../../../packages/scene-assets/blender/skins/nh90.json";
import h1Credits from "../../../../packages/scene-assets/blender/skins/h1.json";
import hindCredits from "../../../../packages/scene-assets/blender/skins/mi_24_mi_35_hind.json";
import hipCredits from "../../../../packages/scene-assets/blender/skins/mi_8_hip.json";
import birdCredits from "../../../../packages/scene-assets/blender/skins/ah_6_mh_6_little_bird.json";
import apacheCredits from "../../../../packages/scene-assets/blender/skins/ah_64_apache.json";
import tigerCredits from "../../../../packages/scene-assets/blender/skins/ec665_tiger.json";
import havocCredits from "../../../../packages/scene-assets/blender/skins/mi_28_havoc.json";
import z20Credits from "../../../../packages/scene-assets/blender/skins/z_20.json";
import blackHawkCredits from "../../../../packages/scene-assets/blender/skins/uh_60_black_hawk.json";

export function SettingsPage() {
  return (
    <>
      <GraphicsSettingsControls onRunAutoDetect={runAutoDetect} />
      <SoundControls />
      <DiagnosticsControls />
      <details>
        <summary>Asset credits</summary>
        {[birdCredits[0], ...h1Credits, ...nh90Credits, ...hindCredits, ...hipCredits, ...apacheCredits, ...blackHawkCredits, ...z20Credits, ...havocCredits, tigerCredits[0]].map((model) => (
          <p key={model.glb}>
            <a href={model.source}>{model.title}</a> by {model.creator}.{" "}
            <a href={model.license_url}>{model.license}</a>. {model.adaptation}
          </p>
        ))}
      </details>
    </>
  );
}
