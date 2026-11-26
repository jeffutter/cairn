# Cairn

Cairn gives your coding agent a way to ask for your review. The agent submits a plan, a design doc, a diagram, an image, or an HTML page. You read it in a browser and comment on specific sections or lines. The agent reads your comments, edits the document, and answers each thread.

Everything runs on your machine. Cairn is one binary with one sqlite file. It has no accounts and no cloud service.

## Install

### From a GitHub release

Each [release](https://github.com/jeffutter/cairn/releases) has a tarball and a `.sha256` file for Linux x86_64 and macOS aarch64 (Apple Silicon). Intel Macs and Linux on ARM have no prebuilt binary; use Nix or build from source.

```sh
# Pick one: linux-x86_64 or macos-aarch64
PLATFORM=macos-aarch64
VERSION=v0.1.0
BASE=https://github.com/jeffutter/cairn/releases/download/$VERSION
curl -fLO $BASE/cairn-$VERSION-$PLATFORM.tar.gz
curl -fLO $BASE/cairn-$VERSION-$PLATFORM.sha256

shasum -a 256 -c cairn-$VERSION-$PLATFORM.sha256   # expect "OK"
tar xzf cairn-$VERSION-$PLATFORM.tar.gz
install -m 755 cairn ~/.local/bin/cairn            # any directory on your PATH
cairn --version
```

The tarball holds the single `cairn` binary. It is not code-signed. If you download it in a browser on macOS and Gatekeeper blocks it, clear the quarantine flag with `xattr -d com.apple.quarantine cairn`. Downloading with `curl` as above does not set it.

### With Nix

The repository is a flake. It supports the default flake systems, so it also builds on Intel Macs and Linux on ARM.

```sh
nix run github:jeffutter/cairn -- serve       # run without installing
nix profile install github:jeffutter/cairn    # install into your profile
nix build github:jeffutter/cairn              # ./result/bin/cairn
```

Pin a release with `github:jeffutter/cairn/v0.1.0`. To use it from another flake, add `inputs.cairn.url = "github:jeffutter/cairn"` and use `inputs.cairn.packages.${system}.default`.

### From source

Build inside `nix develop`, which provides the Mermaid renderer and the htmx script that `build.rs` embeds. A plain `cargo build` outside Nix compiles, but Mermaid diagrams fail at runtime.

## Set up your agent

Agents learn cairn from a skill file. Cairn generates it from its own command definitions, so it matches the version you installed:

```sh
mkdir -p ~/.claude/skills/cairn
cairn skill > ~/.claude/skills/cairn/SKILL.md
```

Any agent that reads Markdown instructions can use the same file. To check whether an installed copy is out of date after an upgrade, run `cairn skill --check ~/.claude/skills/cairn/SKILL.md`. Exit code 1 means regenerate it.

Nix users can link the `skill` package from this flake into the skills directory instead.

Inside Claude Code and pi, cairn prints JSON on its own. For another harness, export `CAIRN_FORMAT=json`.

### Run the server in the background (macOS)

With Home Manager, import the flake's module and enable it:

```nix
imports = [ inputs.cairn.homeManagerModules.default ];
services.cairn.enable = true;
```

launchd then keeps `cairn serve` running on free ports. `cairn status` prints its address, and `cairn view <id>` prints the full link to an artifact. Logs are in `~/Library/Logs/cairn.log`.

## Review a document

1. Start the review UI in a terminal and leave it running:

   ```sh
   cairn serve
   ```

   The UI is at <http://127.0.0.1:7878>.

2. Ask your agent to get a review. For example: "Write the migration plan and submit it to cairn for my review."

3. The agent submits the document and tells you its id. Open the page directly with `cairn view <id>`, or pick it from the list at the UI's home page.

4. Read, then comment. In a markdown document, click a line number to comment on a line range, or "+ section" beside a heading to comment on that section. For images, diagrams, and HTML pages, comments attach to the whole revision.

5. The agent is waiting for comments. When yours arrive, it edits the document, which creates a new revision, and replies in each thread with the revision that addresses it. Replies never resolve a thread. You do that when you are satisfied. The page refreshes itself every two seconds.

6. Use the revision dropdown at the top of the page to compare against earlier versions. An open thread from an older revision stays visible with an "earlier revision" label.

## What you can submit

| Kind | Source | Notes |
| --- | --- | --- |
| markdown | `.md` | Section-level comments. Images are embedded by pointing at other artifacts with `cairn:<id>`. |
| html | one `.html` file or a directory with `index.html` | Runs in a sandboxed frame and must be self-contained. |
| image | png, jpeg, gif, webp | |
| diagram | `.mmd` (Mermaid) or `.svg` | Cairn renders Mermaid to SVG itself. |

Limits: 10 MiB per file, 25 MiB per revision, 200 files per HTML bundle.

## Use the CLI yourself

The agent's commands work for you too. Run `cairn --help` for the full list.

```sh
cairn list                           # artifacts in the current project
cairn list --open                    # only those with open threads
cairn get <id>                       # print the latest revision
cairn comment add <id> "Looks good"  # comment from the terminal
cairn comment list --all             # include resolved threads
cairn delete <id> --yes
```

Output is text on a terminal and JSON in a pipe or an agent session. Force one with `--text` or `--json`.

Cairn scopes artifacts to the git project you run it in. Add `--all-projects` to see everything.

## Where data lives

The database is `$XDG_DATA_HOME/cairn/cairn.db`, usually `~/.local/share/cairn/cairn.db`. Change it with `--db` or `CAIRN_DB`. Deleting an artifact shrinks the file. Cairn never expires or prunes anything on its own.

## Security

The UI and artifact content are served from different ports, so a hostile HTML page cannot act as the review UI. HTML runs in a sandboxed frame with no network access.

`cairn serve` binds to loopback. `--bind 0.0.0.0` makes it reachable from other machines, but cairn has no authentication. Anyone who can reach the port can read and comment. Add `--advertise-host <name>`, the name your browser uses for this machine, so `cairn view` prints a working link.

## More documentation

The design is written down in [`docs/`](docs/README.md): the [CLI reference](docs/cli.md), the [JSON API](docs/api.md) (OpenAPI schema in [`docs/openapi.json`](docs/openapi.json)), the [data model](docs/data-model.md), and the [architecture decisions](docs/adr/). Terms such as artifact, revision, and thread are defined in the [glossary](GLOSSARY.md). Contributors should read [`CLAUDE.md`](CLAUDE.md) for the build and test commands.

## Herdr

[Herdr](https://herdr.dev) is a terminal multiplexer for coding agents. [`herdr/`](herdr/README.md) holds a cairn mod for it: a side panel that lists the current project's artifacts, opens by itself when the agent submits one or revises one, and shows it next to the agent. You can read markdown, images, and diagrams there and comment, reply, resolve, and reopen threads without leaving the terminal. HTML bundles are not drawn (Herdr has no web view), so the panel's **Open** button sends you to the web UI for those.

![Cairn panel in Herdr](herdr/cairn/preview.png)

The mod runs the `cairn` binary, so install cairn first (see [Install](#install)). Then add the mod in one of two ways.

**By hand.** The release tarballs hold only the binary, so get the mod folder from a checkout of this repository:

```sh
git clone https://github.com/jeffutter/cairn
mkdir -p ~/.config/herdr-app/mods
cp -r cairn/herdr/cairn/mods/cairn ~/.config/herdr-app/mods/cairn
herdr-app check ~/.config/herdr-app/mods/cairn
```

The mod finds `cairn` on your `PATH`.

**With Nix.** The flake exports the mod as `packages.<system>.herdr-mod`, with its `cairn` pinned to the same flake's binary. In Home Manager:

```nix
home.file.".config/herdr-app/mods/cairn" = {
  source = "${inputs.cairn.packages.${pkgs.system}.herdr-mod}/mods/cairn";
  recursive = true; # Herdr writes its own files into mod folders
};
```

Either way, the panel also needs a place in your Herdr layout, and that step is manual. Herdr's shipped layouts are read-only, so clone one, add a `cairn` block to the `panel-body` stack, and add a tab-bar button that selects it. The exact layout node and button code are in [`herdr/README.md`](herdr/README.md#what-it-needs). Run `cairn serve` if you want the **Open** button to work; everything else reads the database directly.
