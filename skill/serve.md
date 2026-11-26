`cairn serve` runs the web UI, often as a background service on a port chosen at start. Never guess the URL or the port. After you submit or edit an Artifact, print its full URL and give that URL to the human:

```sh
cairn view abc123
```

`cairn view` prints a URL whether or not a server runs. Check that the URL loads with:

```sh
cairn status
```

It prints the URL of the running server. Exit 3 means no server runs: ask the human to start it with `cairn serve` in a terminal, then run `cairn view` again, because the port can change. Do not run `cairn serve` yourself, because it never returns.
