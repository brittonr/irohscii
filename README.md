# irohscii

ASCII art drawing tool with real-time P2P collaboration via [iroh](https://iroh.computer) and [automerge](https://automerge.org) CRDTs. Inspired by [asciiflow](https://asciiflow.com).

## Run

From GitHub:
```sh
nix run github:brittonr/irohscii
```

Join a session:
```sh
nix run github:brittonr/irohscii -- --join <TICKET>
```

Locally:
```sh
nix run
```

Build with cargo:
```sh
cargo build --release
```

## Options

```
irohscii [OPTIONS] [FILE]

    --join <TICKET>  Join an existing session using a ticket
    --offline        Disable sync (offline mode)
    -h, --help       Print help
    -V, --version    Print version
```

## How it works

Each session generates a shareable ticket. When peers connect, they sync document state using automerge's CRDT protocol over iroh's P2P network. All edits merge automatically without conflicts, and cursor positions are shared in real-time.

## Controls

Use the mouse to draw shapes. Click and drag to create or select.

### Drawing

| Key | Action |
|-----|--------|
| `s` | Select tool |
| `f` | Freehand tool |
| `t` | Text tool |
| `l` | Line tool |
| `a` | Arrow tool |
| `r` | Rectangle tool |
| `b` | Double-border box |
| `d` | Diamond tool |
| `e` | Ellipse tool |
| `v` | Cycle line style |
| `g` | Toggle grid |
| `u` / `U` | Undo / Redo |
| `y` / `p` | Yank / Paste |
| `Del`/`Backspace` | Delete selection |
| Arrow keys | Pan viewport |

### Leader menu (`Space` or `:`)

Press `Space` or `:` to open the leader menu (Helix-style), then a second key:

| Key | Action |
|-----|--------|
| `t` / `Space` | Tool picker |
| `c` | Color picker |
| `b` | Brush picker |
| `s` | Save file |
| `o` | Open file |
| `e` | Export SVG |
| `n` | New document |
| `g` | Toggle grid |
| `l` | Toggle layer panel |
| `p` | Toggle participants panel |
| `T` | Copy sync ticket to clipboard |
| `K` | Connect to cluster |
| `?` / `h` | Help |
| `q` | Quit |

### Other shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+s` | Save (direct) |
| `Ctrl+o` | Open (direct) |
| `Ctrl+c` | Quit (emergency) |
| `Tab` | Session browser |
| `?` / `F1` | Help screen |
