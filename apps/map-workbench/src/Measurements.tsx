import type { Json, JsonObject, Report } from "./protocol";
import { humanize } from "./fields";

function summary(value: Json): string {
  if (Array.isArray(value)) return `[${value.map(summary).join(", ")}]`;
  return typeof value === "number" && !Number.isInteger(value)
    ? String(Number(value.toPrecision(6)))
    : String(value);
}

function Values({ values }: { values: JsonObject | Json[] }) {
  return (
    <dl>
      {Object.entries(values).map(([key, value]) =>
        value &&
        typeof value === "object" &&
        !(Array.isArray(value) && value.every((entry) => typeof entry !== "object")) ? (
          <details key={key}>
            <summary>{humanize(key)}</summary>
            <Values values={value} />
          </details>
        ) : (
          <div key={key}>
            <dt>{humanize(key)}</dt>
            <dd title={Array.isArray(value) ? JSON.stringify(value) : String(value)}>
              {summary(value)}
            </dd>
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
              {name === "Encounter" && <p>Map positions are [x, y] in metres.</p>}
              <Values values={values} />
            </details>
          ),
      )}
    </div>
  );
}
