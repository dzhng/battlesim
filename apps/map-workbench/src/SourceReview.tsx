import { Fragment } from "react";
import { changedPaths, valueAtPath } from "../../fixture-publication/changes";
import type { Replacement } from "./protocol";

export function SourceReview({ file }: { file: Replacement }) {
  const before = JSON.parse(file.before);
  const after = JSON.parse(file.after);
  const changes = changedPaths(before, after);
  return (
    <>
      <h4>Changed values</h4>
      <table className="mw-review-changes">
        <thead>
          <tr>
            <th>Field</th>
            <th>Saved</th>
            <th>Proposed</th>
          </tr>
        </thead>
        <tbody>
          {changes.map((path) => (
            <tr key={path.join(".")}>
              <td>
                <code>
                  {path.map((part, index) => (
                    <Fragment key={index}>
                      {index > 0 && (
                        <>
                          .<wbr />
                        </>
                      )}
                      {part}
                    </Fragment>
                  ))}
                </code>
              </td>
              <td>{JSON.stringify(valueAtPath(before, path)) ?? "absent"}</td>
              <td>{JSON.stringify(valueAtPath(after, path)) ?? "removed"}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="mw-scroll-hint">
        Exact file contents. Scroll inside each pane to inspect the full replacement.
      </p>
      <div className="mw-file-diff">
        <section>
          <h4>Saved file</h4>
          <pre tabIndex={0}>{file.before}</pre>
        </section>
        <section>
          <h4>Proposed file</h4>
          <pre tabIndex={0}>{file.after}</pre>
        </section>
      </div>
    </>
  );
}
