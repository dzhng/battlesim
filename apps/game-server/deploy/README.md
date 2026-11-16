# Server release preparation

A release uses the site's exact simulation and the source and battle data of
**the commit the site's `build.json` names**, regardless of the checkout's HEAD.
That commit must exist in the local repository; preparation never fetches it.
The archive's own bundler builds the lobby and battle artifacts, and each must
state the site's three hashes before any upload. The simulation is downloaded,
never rebuilt. Local edits cannot leak into an archived release.

[The shared identity owner](../../../packages/multiplayer/buildIdentity.ts)
decides which files count. Compatibility is the SHA-256 of the simulation's
bytes, the shared battle documents, and all multiplayer-wire sources. Document
and wire hashes frame sorted repository paths, byte lengths and exact bytes.
The site fixes the latter two at build time; the page hashes the simulation it
actually loads. Its source commit stays in the menu and `build.json` as
provenance, outside admission. Different commits with equal shared hashes play
together.

```sh
bun run deploy:server --site <site-url>
```

The default is a dry run. It verifies identity, prepares an ignored folder and
lists every file it would send, without invoking the host CLI. The folder holds
the hosted lobby, its build identity and [allocation data](settings.json), plus
one gzip-compressed battle archive (`battle.tar.gz`) containing the standalone server, worker and runtime data.
All runtime paths resolve beside the bundles, independently of the working
directory. [The local static-site fixture](localSite.testSupport.ts) publishes
the existing simulation bytes with this same layout for the deploy rehearsal;
it needs no browser build or network access.

[The hosted entry](lobby.ts) supplies [the sandbox provider](sandboxes.ts) to the
same lobby startup used locally. Only this directory knows Bento's service,
credentials and launch environment. The app receives `PORT`, `BENTO_API_URL`
and `BENTO_APP_API_KEY` from its host. A battle receives only its key, bind host
and port through the sandbox's create environment; command history never holds
credentials. The public battle port authenticates control requests with the
battle key; player routes still require acknowledged seating.

Provider startup stops leftovers labelled as this game's battles before the
lobby accepts traffic. This assumes one lobby per workspace: restarting it ends
its old battles. Stop captures redacted command diagnostics before requesting
sandbox shutdown. Deletion waits asynchronously for archival; the finite rental
bounds an orphan even if the lobby or service disappears. Capacity and rental
length are provisional until the host measurements are complete.

The push path remains explicit:

```sh
bun run deploy:server --site <site-url> --push
```

[Settings](settings.json) own the live app and workspace, with their rationale;
there are no app/workspace flags or environment overrides. The archived settings
travel with the release. The [host adapter](push.ts) creates folders before
writing files and registers an app with service access. A second deploy replaces
files, updates runtime settings and restarts the existing app. It never changes
who can reach the lobby; making the app public is the operator's step. Its
injected runner is proven against a local interpreter of the CLI's endpoint
schemas. No port is chosen for the lobby, and no exported credential is needed.

```sh
bun run deploy:server --site <site-url> --check
```

This read-only mode compares the site's identity with the live lobby's public
`GET /build`. It exits 0 when admitted, 2 when deployment is needed (including
an unreachable or older lobby), and 1 if the site cannot be verified. The route
returns only the three hashes: unlike `/hello`, it creates no player, callsign
or presence and takes no credential. Neither check nor dry preparation uploads.

Promotion calls this mode after publishing the site, then `--push` only when
needed, keeping ongoing battles alone when the shared build agrees. Both calls
also supply `--commit <promoted-sha>` to refuse a site that names another source.
A failed deploy leaves the promoted site in place and promotion exits nonzero
with a single repair command. Rerunning promotion with approval also repairs an
already promoted site.

The hosted lobby is told which header carries a client's address by
[the host settings](settings.json). A header belongs there only when the host's
proxy overwrites whatever a caller sent for it; that was checked for the value
there. A request that lacks the header is refused rather than mixing address
sources. A local lobby has no proxy and is told none.
