import type { Json, JsonObject, Report } from "./protocol";
import { humanize } from "./fields";

function Values({ values }: { values: JsonObject | Json[] }) {
  return (
    <dl>
      {Object.entries(values).map(([key, value]) =>
        value && typeof value === "object" ? (
          <details key={key}>
            <summary>{humanize(key)}</summary>
            <Values values={value} />
          </details>
        ) : (
          <div key={key}>
            <dt>{humanize(key)}</dt>
            <dd>{String(value)}</dd>
          </div>
        ),
      )}
    </dl>
  );
}

export function Measurements({ report }: { report: Report }) {
  const groups = { Objects: report.counts, Geometry: report.metrics, Encounter: report.encounter };
  return (
    <div className="mw-measurements">
      {Object.entries(groups).map(
        ([name, values]) =>
          values && (
            <details key={name} open={name === "Objects"}>
              <summary>{name}</summary>
              <Values values={values} />
            </details>
          ),
      )}
    </div>
  );
}
