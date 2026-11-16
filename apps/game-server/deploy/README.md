# Server release preparation

A release uses the site's exact simulation and the checkout's committed source
and game data. It never rebuilds the simulation. The site publishes `build.json`
and `sim/`; mismatched commits or hashes stop preparation before bundling. Local
edits cannot leak into an archived release, so commit implementation changes
before rehearsing it.

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
bun run deploy:server --site <site-url> --push --app <app-slug> --workspace <workspace>
```

`BENTO_WORKSPACE` may supply the workspace instead of the flag. The
[host adapter](push.ts) creates folders before writing files, registers an app
with service access and makes its route public. A second deploy replaces files,
updates runtime settings and restarts the existing app. Its injected runner is
proven against a local interpreter of the installed CLI's endpoint schemas;
real service acceptance remains a deployment check. No port is chosen for the
lobby, and no exported credential is needed.

The public proxy's client-address header is still unverified. Until it is
verified to replace caller values, leave `CLIENT_ADDRESS_HEADER` unset: the
lobby uses its socket address and ignores forwarded headers. A configured
header must be present on room streams; missing values are refused rather than
mixing address sources. Publishing a site address and promoting a site remain
separate decisions.
