// What a browser the game doesn't run in shows in its place
// (`web/src/browserSupport.ts`): the loading screen's plate, saying to open
// the page in Chrome, with this address ready to copy there. Nothing of the
// game loads behind it.
import { useState } from "react";
import { Wordmark } from "./Wordmark";

export function UnsupportedBrowser() {
  const [copied, setCopied] = useState(false);
  const copy = () =>
    void navigator.clipboard.writeText(location.href).then(
      () => setCopied(true),
      () => {},
    );
  return (
    <main className="menu loading" data-testid="unsupported-browser">
      <div className="hud-panel menu-body">
        <h1>
          <Wordmark />
        </h1>
        <div className="loading-failure" role="alert">
          <p className="hud-error">Open this page in Google Chrome</p>
          <p className="loading-advice">
            This browser can't draw the battle. Copy the link and open it in Chrome on a desktop or
            laptop.
          </p>
          <div className="loading-actions">
            <button type="button" className="hud-menu-item" onClick={copy} autoFocus>
              {copied ? "Link copied" : "Copy link"}
            </button>
            <a className="hud-menu-item" href="https://www.google.com/chrome/">
              Get Chrome
            </a>
          </div>
        </div>
      </div>
    </main>
  );
}
