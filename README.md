# irohscii

ASCII art drawing tool with real-time P2P collaboration via [iroh](https://iroh.computer) and [automerge](https://automerge.org) CRDTs. Inspired by [asciiflow](https://asciiflow.com).

## Core Principles

| Principle | Implementation |
|-----------|---------------|
| **Local-first** | Automerge document is the source of truth locally; works offline |
| **P2P / no server** | Iroh provides direct peer connections via QUIC |
| **Conflict-free** | CRDTs ensure all edits merge automatically, no conflicts ever |
| **Real-time** | Presence (cursors, activity) syncs at 20 Hz; document syncs at 5 Hz |

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

## How It Works

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

### Leader Menu (`Space` or `:`)

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

### Other Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+s` | Save (direct) |
| `Ctrl+o` | Open (direct) |
| `Ctrl+c` | Quit (emergency) |
| `Tab` | Session browser |
| `?` / `F1` | Help screen |

---

## Architecture

### Crate Structure

The project is a Rust workspace with 5 library crates + 1 binary crate:

```
irohscii (workspace root — binary crate)
├── crates/
│   ├── irohscii-geometry    # Pure geometry: line drawing, shape rendering, viewport
│   ├── irohscii-core        # Document model, shapes, layers (Automerge-backed)
│   ├── irohscii-sync        # P2P sync: Iroh networking + Automerge sync + presence
│   ├── irohscii-export      # ASCII and SVG export
│   └── irohscii-session     # Session management, undo history
└── src/                     # Binary: TUI app, tools, modes, UI rendering
```

### Dependency Graph

```
irohscii-geometry          (zero dependencies — pure math)
       ↑
irohscii-core              (depends on geometry + automerge)
       ↑
irohscii-sync              (depends on core + iroh + aspen-automerge)
irohscii-export            (depends on core + geometry)
irohscii-session           (depends on core + automerge)
       ↑
irohscii (binary)          (depends on ALL crates + ratatui + crossterm)
```

- **Separation of concerns**: Geometry is pure math (testable with proptest). Core is the data model. Sync is networking. UI is rendering.
- **`irohscii-geometry` has zero external dependencies** — pure algorithms, highly testable and reusable.
- **`irohscii-core` owns the Automerge document** — every shape mutation goes through it, ensuring CRDT consistency.

---

## Key Technologies

### Iroh (P2P Networking)

Iroh is a P2P networking library built on QUIC (via Quinn). It provides:

- **Cryptographic identity**: Every peer gets a unique Ed25519 keypair (public key = peer identity).
- **Endpoint discovery**: Peers find each other via DNS discovery and Pkarr (DHT publishing).
- **NAT traversal**: Relay servers for hole punching; falls back to relaying if direct connection fails.
- **ALPN-based protocol routing**: Multiple protocols multiplex over the same QUIC connection, distinguished by ALPN strings.

**EndpointAddr** bundles the peer's public key + relay URL + direct addresses. This is serialized into a ticket string (base32-encoded, prefixed with `irohscii1`) that users share to connect.

**Protocol routing** uses iroh's `Router` to dispatch incoming connections by ALPN — document sync and presence sync are separate protocols on different QUIC streams over the same connection infrastructure.

### Automerge (CRDTs)

Automerge is a CRDT library providing a JSON-like document model where:

- Multiple users can edit simultaneously
- Edits never conflict — they merge deterministically
- No central server needed for coordination
- Works offline; syncs when connectivity resumes

CRDTs guarantee **strong eventual consistency**: if two peers have seen the same set of changes, they'll have identical document state regardless of the order changes were applied.

By using UUIDs for shape IDs, two users creating shapes simultaneously create entries at different keys, so they never conflict. Concurrent edits to the same property use last-writer-wins with deterministic tiebreaking.

---

## Document Model

The `Document` struct wraps an `Automerge` instance. The document is structured as a nested map:

```
ROOT
├── "id"          → DocumentId (UUID string)
├── "shapes"      → Map { shape_uuid → Map { kind, start_x, start_y, ... } }
├── "shape_order" → List [ shape_uuid, shape_uuid, ... ]  (render order, bottom→top)
├── "groups"      → Map { group_uuid → Map { members: List, parent? } }
├── "layers"      → Map { layer_uuid → Map { name, visible, locked } }
├── "layer_order" → List [ layer_uuid, ... ]
├── "undo_stack"  → List [ serialized_snapshot_bytes, ... ]
└── "redo_stack"  → List [ serialized_snapshot_bytes, ... ]
```

Every mutation goes through Automerge transactions, ensuring CRDT consistency.

### Shape Types

| Shape | Key Fields |
|-------|-----------|
| `Line` | start, end, style, connections, label, color |
| `Arrow` | start, end, style, connections, label, color |
| `Rectangle` | start, end, label, color |
| `DoubleBox` | start, end, label, color |
| `Diamond` | center, half_width, half_height, label, color |
| `Ellipse` | center, radius_x, radius_y, label, color |
| `Triangle` | p1, p2, p3, label, color |
| `Freehand` | points (Vec), char, label, color |
| `Text` | pos, content, color |
| `Parallelogram` | start, end, label, color |
| `Hexagon` | center, radius_x, radius_y, label, color |
| `Trapezoid` | start, end, label, color |
| `RoundedRect` | start, end, label, color |
| `Cylinder` | start, end, label, color |
| `Cloud` | start, end, label, color |
| `Star` | center, outer_radius, inner_radius, label, color |

### Layers

- Each shape belongs to exactly one layer
- Layers can be visible/hidden and locked/unlocked
- Layer order determines rendering order
- Deleting a layer moves its shapes to the default layer

### Groups

- Selecting one shape in a group selects all
- Moving one moves all
- Supports nested groups (parent chain)
- Stored in Automerge as a separate `groups` map

### Connections

Lines/arrows can snap to shapes and maintain connections:
- Each line endpoint stores an optional connection (shape UUID)
- When a connected shape moves, connected lines update their endpoints
- Snap points are the midpoints of shape edges (top, bottom, left, right)

---

## Sync Architecture

The sync system runs on a **dedicated thread** with its own tokio runtime, communicating with the main thread via channels.

### Thread Model

```
┌─────────────────────────────────────────┐
│              Main Thread                 │
│  (crossterm events → App mutations →     │
│   render via ratatui)                    │
│                                          │
│  SyncCommand ──→ [command_tx] ──→        │
│                                   ↓      │
│  ←── [event_rx] ←── SyncEvent           │
└─────────────────────────────────────────┘
                    ↕
┌─────────────────────────────────────────┐
│           Sync Thread (tokio)            │
│                                          │
│  ┌── iroh Endpoint ──────────────────┐  │
│  │  ├── Router                        │  │
│  │  │   ├── ALPN: automerge/sync/1   │  │
│  │  │   │   (AutomergeSyncHandler)    │  │
│  │  │   └── ALPN: irohscii/presence/1│  │
│  │  │       (PresenceProtocol)        │  │
│  │  ├── Persistent Peer Connection    │  │
│  │  └── Persistent Cluster Connection │  │
│  └────────────────────────────────────┘  │
│                                          │
│  LocalDocumentStore (in-memory)          │
│    ├── docs: HashMap<key, Vec<u8>>       │
│    └── change_tx → notifies on sync      │
└─────────────────────────────────────────┘
```

The main thread is synchronous (crossterm event loop), so `std::sync::mpsc` is used for the cross-thread interface. The sync thread uses `tokio::sync::mpsc` internally.

### Commands & Events

**Commands (Main → Sync)**:

| Command | Purpose |
|---------|---------|
| `SyncDoc { doc }` | Push local document changes to peers |
| `BroadcastPresence(presence)` | Send cursor/activity to peers |
| `ConnectCluster { ticket }` | Connect to an aspen cluster node |
| `Shutdown` | Clean shutdown |

**Events (Sync → Main)**:

| Event | Purpose |
|-------|---------|
| `Ready { endpoint_id, local_peer_id }` | Connection established |
| `RemoteChanges { doc }` | Received changes from peers |
| `PresenceUpdate(presence)` | Remote cursor moved |
| `PresenceRemoved { peer_id }` | Peer disconnected |
| `Error(msg)` | Something went wrong |

### Sync Timing

| Interval | What | Why |
|----------|------|-----|
| 200ms | Periodic sync with peer | Pull remote changes regularly |
| 2s | Periodic sync with cluster | Persistence is less urgent |
| 50ms | Presence broadcast | Smooth cursor tracking (20 Hz) |
| 16ms | Event loop poll | ~60 FPS rendering |

### LocalDocumentStore

The bridge between the sync protocol and the application:

- **`update_from_app()`**: App pushes local changes. Does NOT notify (avoids feedback loop).
- **`save()` (from sync)**: Sync protocol saves merged doc. DOES notify if content changed.
- Change detection: only sends notifications when bytes actually differ.

### Ticket System

```
irohscii1ABCDEF...  →  base32(postcard(EndpointAddr))
                        Contains: PublicKey + relay URL + direct addresses

amsync1ABCDEF...    →  Aspen sync ticket (address + capability token)
                        Used for authenticated cluster connections
```

### Connection Model

- **Peer sync**: One persistent QUIC connection reused for all syncs. Each `sync_with_peer()` opens a new bidirectional stream (QUIC multiplexing).
- **Cluster sync**: Separate persistent connection to an aspen cluster node for durable storage.
- **Presence**: Separate QUIC connection with different ALPN, running a continuous bidirectional loop.

---

## Presence System

Presence shows where other users' cursors are and what they're doing in real-time. It's separate from document sync because presence data is ephemeral (not persisted), high-frequency (20 Hz), and small.

### PeerPresence Data

```rust
pub struct PeerPresence {
    pub peer_id: PeerId,
    pub cursor_pos: Position,
    pub activity: CursorActivity,
    pub color_index: u8,              // Deterministic color assignment
    pub timestamp_ms: u64,
    pub active_layer_id: Option<LayerId>,
    pub drag_start_ms: Option<u64>,   // For soft lock priority
}
```

### Cursor Activity States

| Activity | What's Shown | Extra Data |
|----------|-------------|-----------|
| `Idle` | Colored cursor block | — |
| `Drawing` | Ghost preview of shape | tool, start, current position |
| `Selected` | Colored corners around shape | shape_id |
| `Dragging` | Dashed ghost rectangle at new position | shape_id, delta |
| `Resizing` | Dashed ghost rectangle at new bounds | shape_id, preview_bounds |
| `Typing` | Blinking cursor indicator | position |

### Soft Lock

When two peers drag the same shape simultaneously, the peer who started earlier gets priority (first-dragger-wins via `drag_start_ms`). The other peer sees the first peer's ghost and can choose to let go. This is a soft lock — visual feedback, no hard prevention.

### Wire Format

```
[4 bytes: length LE u32][msgpack-encoded PresenceMessage]
```

Peers that haven't sent presence in 5 seconds are pruned from the active list.

---

## Session Management

Sessions provide named workspaces for organizing multiple drawings.

### Storage Layout

```
~/.local/share/irohscii/
├── sessions/
│   ├── my-diagram/
│   │   ├── document.automerge
│   │   └── meta.json
│   └── architecture-sketch/
│       ├── document.automerge
│       └── meta.json
└── session_registry.json
```

### Lifecycle

1. **Create**: Creates directory, empty doc, meta.json
2. **Open**: Loads automerge doc + meta from disk
3. **Save**: Writes doc bytes + meta JSON
4. **Switch**: Save current → load new → reset UI state
5. **Delete**: Remove directory + update registry

---

## Undo/Redo System

Automerge doesn't support rollback — it's an append-only CRDT where every change is permanent. irohscii uses **snapshot-based undo**: serialize all shapes before each mutation, store on undo/redo stacks, and restore on undo.

### Two Implementations

1. **Global (CRDT-synced)**: Stored in the Automerge document (`undo_stack`/`redo_stack` lists). Syncs across peers. Max 50 entries.
2. **Local (per-session)**: `UndoManager` with memory or disk-backed storage. Unlimited history via disk snapshots. NOT synced.

### Disk-Backed Storage

```
~/.local/share/irohscii/undo/{session_id}/
├── 00000000.snap
├── 00000001.snap
└── ...
```

Memory cache of 20 most recent snapshots for performance.

---

## Rendering & UI

The UI uses **ratatui** with **crossterm** as the terminal backend.

### Layout

```
┌──────────────────────────────────────────────────────┐
│                    Canvas Area                        │
│  (infinite, pannable, zoomable)                      │
│                                                       │
│  ┌─ Layer Panel ─┐  ┌─ Participants ─┐              │
│  │ ● Layer 1     │  │ ● You  [L1]    │              │
│  │   Layer 2     │  │ █ Peer-ab (Idle)│              │
│  └───────────────┘  └────────────────┘              │
├──────────────────────────────────────────────────────┤
│ SEL  [unsaved]  [3]  [SYNC 1]               Default │
├──────────────────────────────────────────────────────┤
│ click to select | [Space] menu [?] help              │
└──────────────────────────────────────────────────────┘
```

### Render Order (bottom to top)

1. Grid dots (if enabled)
2. Snap guide lines
3. All shapes (via ShapeView cache, respecting z-order and layer visibility)
4. Freehand preview (while drawing)
5. Snap point diamonds (when line/arrow tool active)
6. Shape preview (while drawing)
7. Active snap indicator
8. Marquee selection box
9. Selection bounding boxes + resize handles
10. Remote cursors + activity ghosts (always on top)

### Mode System

Modal UI inspired by Vim/Helix. Modes include: Normal, TextInput, LabelInput, PathInput, RecentFiles, SelectionPopup, ConfirmDialog, HelpScreen, SessionBrowser, SessionCreate, KeyboardShapeCreate, LayerRename.

Each mode implements `handle_key()` returning a `ModeTransition` (Stay, Normal, To, or Action). The mode is taken out of `App` with `std::mem::take()` before handling to avoid borrow checker issues with simultaneous `&mut mode` and `&mut app`.

### ShapeView (Render Cache)

A read-only cache rebuilt from the Automerge document after every mutation. Avoids deserializing shapes from Automerge 60 times per second during rendering. Provides O(1) shape lookup by ID and spatial indexing for hit-testing.

---

## Shape System & Geometry

### ASCII Art Rendering

Each shape type has a function returning `Vec<(Position, char)>` — rectangles use box-drawing characters (`┌──┐│└──┘`), lines use directional characters (`─│/\`), and ellipses use Bresenham's algorithm adapted for ASCII.

### Line Styles

| Style | Description |
|-------|-------------|
| `Straight` | Direct line: `─ │ / \` |
| `OrthogonalHV` | Horizontal first, then vertical: `──┐` |
| `OrthogonalVH` | Vertical first, then horizontal: `│└──` |
| `OrthogonalAuto` | Auto-routed to avoid obstacles |

### Viewport (Infinite Canvas)

The canvas is infinite — positions can be negative. The `Viewport` handles pan/zoom (0.25x to 4.0x), converting between screen coordinates (mouse clicks) and canvas coordinates (shape positions).

### Snap Points

Shapes expose snap points (midpoints of edges) that lines/arrows connect to. Drawing near a shape edge (within 3 characters) snaps to the nearest point, creating connections that update when shapes move.

---

## Data Flow

### Drawing a Shape

```
1. Mouse down       → app.start_shape(pos)         [ShapeState created]
2. Mouse move       → app.update_shape(pos)         [Preview renders in yellow]
3. Mouse up         → app.commit_shape()
   a. save_undo_state()                              [Snapshot → undo_stack]
   b. doc.add_shape(ShapeKind::Rectangle{...})       [Automerge transaction]
   c. shape_view.rebuild(&doc)                        [Cache rebuilt]
   d. doc.mark_dirty()                                [Triggers autosave]
4. Sync thread      → SyncDoc sent to peer           [Automerge bytes pushed]
```

### Receiving a Remote Edit

```
1. Sync protocol    → Remote peer sends sync message
2. LocalDocumentStore::save()                         [Merged doc saved, change_tx fires]
3. SyncEvent::RemoteChanges { doc }                   [Event → main thread]
4. app.merge_remote(&mut doc)
   a. self.doc.merge(other)                           [Automerge merge — conflict-free]
   b. self.shape_view.rebuild(&doc)                   [Cache rebuilt with new shapes]
5. Next frame       → UI renders with merged state
```

### Presence Update

```
1. Mouse moves          → cursor_pos updated
2. Every 50ms           → app.build_presence(cursor_pos)
3. SyncCommand::BroadcastPresence(presence)
4. presence_protocol.broadcast(presence)              [To all peers]
5. Remote peer receives → SyncEvent::PresenceUpdate
6. presence_manager.update_peer(presence)
7. Next frame           → render_remote_cursor()      [Colored cursor + ghost]
```

---

## Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Automerge doc is the source of truth** | Sync is trivial (just merge docs), no separate model to keep in sync. Trade-off: cache rebuild on every edit. |
| **Separate sync thread** | Main thread never blocked by network I/O. Clean channel-based separation, no shared mutable state. |
| **Presence as separate protocol** | Avoids polluting permanent CRDT history with ephemeral, high-frequency cursor data. |
| **Snapshot-based undo** | Automerge has no rollback. Serialized snapshots stored in the CRDT so undo/redo syncs across peers. |
| **UUID shape identity** | No ID collisions when peers create shapes simultaneously. No coordination needed. |
| **Persistent QUIC connections** | QUIC multiplexed streams are cheap. Reduces connection overhead, handles NAT keepalive. |

### Serialization Formats

| Format | Usage |
|--------|-------|
| Automerge binary | Document storage/sync (compact, CRDT-native) |
| MessagePack | Presence messages (compact binary, fast) |
| Postcard | Ticket encoding (compact, no-std compatible) |
| JSON | Session metadata (human-readable config) |
| Base32 | Ticket strings (URL-safe, case-insensitive) |
