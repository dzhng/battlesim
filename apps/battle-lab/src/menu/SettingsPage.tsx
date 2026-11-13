// The menu's Settings page: graphics, sound and diagnostics.
import { DiagnosticsControls } from "../DiagnosticsControls";
import { GraphicsSettingsControls } from "../GraphicsSettingsControls";
import { SoundControls } from "../SoundControls";
import { runAutoDetect } from "../graphicsDetection";

export function SettingsPage() {
  return (
    <>
      <GraphicsSettingsControls onRunAutoDetect={runAutoDetect} />
      <SoundControls />
      <DiagnosticsControls />
    </>
  );
}
