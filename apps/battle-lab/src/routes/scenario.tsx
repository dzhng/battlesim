import { useLocation } from "react-router";
import ScenarioBuilder from "../scenario-builder/ScenarioBuilder";
import ScenarioPlay from "./scenarioPlay";

/** One lab identity owns authoring and the real upload-to-worker handoff. */
export default function Scenario() {
  const { search } = useLocation();
  return new URLSearchParams(search).get("mode") === "play" ? <ScenarioPlay /> : <ScenarioBuilder />;
}
