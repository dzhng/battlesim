# Server release preparation

A server release uses the site's simulation bytes and the checkout's committed
source and game data. It never rebuilds the simulation. The site must publish
`build.json` and `sim/`; mismatched commits and hashes stop preparation before
bundling. The source archive makes local edits irrelevant to the release.

From the repository root:

```sh
bun run deploy:server --site <site-url>
```

The default is a dry run: it verifies, prepares an ignored folder, and lists
every file it would send. It invokes no host command. Keep that folder for
inspection; startup reads its `build.json` and data beside the entry point,
independently of the working directory.

The untested host adapter is [bento.mjs](bento.mjs). Its local CLI help supplied
the file-write and private-app-registration commands. The proxy header is
unconfigured: the help does not establish it, and no host request was made.
Verify it before address-based throttling relies on it. No public visibility or promotion is part of this step.

Only after the repository owner authorizes an outward release:

```sh
bun run deploy:server --site <site-url> --push --app <new-app-slug>
```

That path has never been run. The account and workspace come from the operator's
CLI configuration; neither belongs in repository data. Existing app updates,
public-route measurements and publication are separate work.
