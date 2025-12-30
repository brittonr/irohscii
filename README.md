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

| Key | Action |
|-----|--------|
| Arrow keys | Pan viewport |
| `Del`/`Backspace` | Delete selection |
| `s` | Select tool |
| `f` | Freehand tool |
| `t` | Text tool |
| `l` | Line tool |
| `a` | Arrow tool |
| `r` | Rectangle tool |
| `b` | Double-border box |
| `d` | Diamond tool |
| `e` | Ellipse tool |
| `c` | Cycle brush character |
| `v` | Cycle line style |
| `g` | Toggle grid |
| `u` / `U` | Undo / Redo |
| `y` / `p` | Yank / Paste |
| `T` | Copy ticket to clipboard |
| `P` | Toggle participants panel |
| `E` | Export to SVG |
| `Ctrl+s` | Save |
| `Ctrl+o` | Open |
| `N` | New document |
| `q` | Quit |
