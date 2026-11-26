# Find the running server through a unix socket beside the database

`cairn serve` can run as a background service on ports the OS picks (`--port 0`), so a CLI cannot assume where the web UI is. `serve` therefore listens on a unix socket at `<database path>.sock`, and `cairn status` and `cairn view` ask it for the server's URLs and ports.

The path is derived from the database path, so the server and a CLI that would write the same database always agree on it, with no flag. The protocol is one request line (`status`) and one JSON response line per connection; an unknown request gets `{"error": ...}`. An exclusive lock on `<socket>.lock`, held while the server runs, makes one server per database and lets a new server safely replace the socket a killed one left behind.

We rejected a port file: it goes stale when the server dies, and a reader cannot tell. This does not change [ADR 0001](0001-single-binary-direct-database-access.md): the socket carries only the server's address, and the CLI still reads and writes the database directly, with or without a server.

Consequences: a second `serve` on the same database exits 1, even on other ports; `serve` needs a filesystem that allows unix sockets and a database path short enough for one (about 100 bytes).
