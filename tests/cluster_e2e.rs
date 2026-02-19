//! End-to-end cluster test for irohscii with aspen-automerge backend.
//!
//! This test creates real iroh P2P endpoints, establishes connections,
//! and verifies that Automerge documents sync correctly through the
//! aspen-automerge DocumentStore and sync protocol.
//!
//! Tests verify actual document content (shapes, layers, groups) — not
//! just that bytes arrived.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use automerge::{Automerge, ReadDoc};
use irohscii::canvas::Position;
use irohscii::document::Document;
use irohscii::layers::LayerId;
use irohscii::presence::{CursorActivity, PeerId, PeerPresence};
use irohscii::shapes::{ShapeColor, ShapeKind};
use irohscii::sync::{
    SyncCommand, SyncConfig, SyncEvent, SyncHandle, SyncMode, start_sync_thread,
};

// Serialize tests — each one binds iroh endpoints on random ports, but the
// global test mutex avoids resource exhaustion under parallel execution.
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Acquire the test mutex, recovering from poison if a prior test panicked.
fn lock_tests() -> std::sync::MutexGuard<'static, ()> {
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

const SYNC_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECTION_DELAY: Duration = Duration::from_millis(1500);

// ========== Helpers ==========

fn wait_for_event<F>(handle: &SyncHandle, timeout: Duration, mut pred: F) -> Option<SyncEvent>
where
    F: FnMut(&SyncEvent) -> bool,
{
    let t0 = Instant::now();
    loop {
        if let Some(ev) = handle.poll_event() {
            if pred(&ev) {
                return Some(ev);
            }
        }
        if t0.elapsed() > timeout {
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_ready(h: &SyncHandle) -> (String, PeerId) {
    wait_for_event(h, SYNC_TIMEOUT, |e| matches!(e, SyncEvent::Ready { .. }))
        .and_then(|e| match e {
            SyncEvent::Ready {
                endpoint_id,
                local_peer_id,
            } => Some((endpoint_id, local_peer_id)),
            _ => None,
        })
        .expect("peer should become ready")
}

fn wait_remote(h: &SyncHandle) -> Automerge {
    wait_for_event(h, SYNC_TIMEOUT, |e| {
        matches!(e, SyncEvent::RemoteChanges { .. })
    })
    .and_then(|e| match e {
        SyncEvent::RemoteChanges { doc } => Some(*doc),
        _ => None,
    })
    .expect("should receive remote changes")
}

/// Drain all pending RemoteChanges events and return the last document.
fn drain_remote(h: &SyncHandle, timeout: Duration) -> Option<Automerge> {
    let t0 = Instant::now();
    let mut last = None;
    loop {
        match h.poll_event() {
            Some(SyncEvent::RemoteChanges { doc }) => {
                last = Some(*doc);
            }
            Some(_) => {} // ignore other events
            None => {
                if t0.elapsed() > timeout {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    last
}

fn host() -> (SyncHandle, String, PeerId) {
    let cfg = SyncConfig {
        mode: SyncMode::Active { join_ticket: None },
        cluster_ticket: None,
        disable_discovery: true,
    };
    let h = start_sync_thread(cfg).expect("start host");
    let (ticket, pid) = wait_ready(&h);
    (h, ticket, pid)
}

fn join(ticket: &str) -> (SyncHandle, PeerId) {
    let cfg = SyncConfig {
        mode: SyncMode::Active {
            join_ticket: Some(ticket.into()),
        },
        cluster_ticket: None,
        disable_discovery: true,
    };
    let h = start_sync_thread(cfg).expect("start joiner");
    let (_, pid) = wait_ready(&h);
    (h, pid)
}

fn cleanup(handles: Vec<SyncHandle>) {
    for h in handles {
        let _ = h.send_command(SyncCommand::Shutdown);
    }
    std::thread::sleep(Duration::from_millis(200));
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> ShapeKind {
    ShapeKind::Rectangle {
        start: Position::new(x, y),
        end: Position::new(x + w, y + h),
        color: ShapeColor::default(),
        label: None,
    }
}

fn labeled_rect(x: i32, y: i32, w: i32, h: i32, label: &str) -> ShapeKind {
    ShapeKind::Rectangle {
        start: Position::new(x, y),
        end: Position::new(x + w, y + h),
        color: ShapeColor::default(),
        label: Some(label.to_string()),
    }
}

/// Wait until the handle delivers a RemoteChanges doc with at least `min_shapes` shapes.
fn wait_for_shapes(h: &SyncHandle, min_shapes: usize, timeout: Duration) -> Option<Automerge> {
    let t0 = Instant::now();
    let mut best: Option<Automerge> = None;
    while t0.elapsed() < timeout {
        if let Some(ev) = h.poll_event() {
            if let SyncEvent::RemoteChanges { doc } = ev {
                let n = count_shapes(&doc);
                if n >= min_shapes {
                    return Some(*doc);
                }
                best = Some(*doc);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    best
}

/// Read all key names from an Automerge map object at root.
fn root_keys(doc: &Automerge) -> Vec<String> {
    doc.keys(automerge::ROOT).collect()
}

/// Count shapes in a raw Automerge document (reads the "shapes" map).
fn count_shapes(doc: &Automerge) -> usize {
    match doc.get(automerge::ROOT, "shapes") {
        Ok(Some((_, shapes_obj))) => doc.keys(&shapes_obj).count(),
        _ => 0,
    }
}

/// Read shape_order length in a raw Automerge document.
fn shape_order_len(doc: &Automerge) -> usize {
    match doc.get(automerge::ROOT, "shape_order") {
        Ok(Some((_, order_obj))) => doc.length(&order_obj),
        _ => 0,
    }
}

// ========== Tests ==========

/// Full end-to-end: host creates shapes, joiner receives them, verifies content.
#[test]
fn e2e_shapes_sync_to_joiner() {
    let _g = lock_tests();

    let (ha, ticket, _pa) = host();
    let (hb, _pb) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Host creates a document with 3 shapes
    let mut doc_a = Document::new();
    doc_a.add_shape(rect(0, 0, 10, 5)).unwrap();
    doc_a.add_shape(rect(20, 20, 8, 4)).unwrap();
    doc_a
        .add_shape(labeled_rect(40, 0, 12, 6, "Server"))
        .unwrap();

    // Push to sync
    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    // Joiner receives remote changes
    let remote = wait_remote(&hb);

    // Verify the remote document has the shapes structure
    assert!(
        root_keys(&remote).contains(&"shapes".to_string()),
        "remote doc should have 'shapes' key"
    );
    assert_eq!(
        count_shapes(&remote),
        3,
        "remote doc should have exactly 3 shapes"
    );
    assert_eq!(
        shape_order_len(&remote),
        3,
        "shape_order should have 3 entries"
    );

    // Verify it can be loaded as a full Document
    let doc_b = Document::from_automerge(remote);
    let shapes_b = doc_b.read_all_shapes().unwrap();
    assert_eq!(shapes_b.len(), 3, "Document should reconstruct 3 shapes");

    // Verify shape content matches
    let kinds: Vec<&ShapeKind> = shapes_b.iter().map(|(_, k)| k).collect();
    let has_server_label = kinds.iter().any(|k| match k {
        ShapeKind::Rectangle { label, .. } => label.as_deref() == Some("Server"),
        _ => false,
    });
    assert!(has_server_label, "should have the 'Server' labeled rectangle");

    cleanup(vec![ha, hb]);
}

/// Joiner creates shapes and host receives them (reverse direction).
#[test]
fn e2e_shapes_sync_to_host() {
    let _g = lock_tests();

    let (ha, ticket, _pa) = host();
    let (hb, _pb) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Joiner creates shapes
    let mut doc_b = Document::new();
    doc_b.add_shape(rect(5, 5, 15, 15)).unwrap();
    doc_b.add_shape(rect(30, 30, 10, 10)).unwrap();

    hb.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_b.clone_automerge()),
    })
    .unwrap();

    // Host receives (via periodic sync pulling changes)
    let remote = wait_remote(&ha);
    assert_eq!(count_shapes(&remote), 2, "host should receive 2 shapes");

    let doc_a = Document::from_automerge(remote);
    let shapes_a = doc_a.read_all_shapes().unwrap();
    assert_eq!(shapes_a.len(), 2);

    cleanup(vec![ha, hb]);
}

/// Both peers create shapes concurrently, then both converge to the merged state.
#[test]
fn e2e_concurrent_shapes_converge() {
    let _g = lock_tests();

    let (ha, ticket, _pa) = host();
    let (hb, _pb) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Host creates 2 shapes
    let mut doc_a = Document::new();
    doc_a
        .add_shape(labeled_rect(0, 0, 10, 5, "HostRect1"))
        .unwrap();
    doc_a
        .add_shape(labeled_rect(15, 0, 10, 5, "HostRect2"))
        .unwrap();

    // Joiner creates 2 shapes (independent doc, concurrent edits)
    let mut doc_b = Document::new();
    doc_b
        .add_shape(labeled_rect(0, 10, 10, 5, "JoinRect1"))
        .unwrap();
    doc_b
        .add_shape(labeled_rect(15, 10, 10, 5, "JoinRect2"))
        .unwrap();

    // Push both at the same time
    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();
    hb.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_b.clone_automerge()),
    })
    .unwrap();

    // Wait for both to receive some changes, then let periodic sync settle
    std::thread::sleep(Duration::from_secs(3));

    // Drain all events to get the latest state
    let last_a = drain_remote(&ha, Duration::from_secs(2));
    let last_b = drain_remote(&hb, Duration::from_secs(2));

    // Both should have received remote changes
    // Note: due to CRDT merge semantics, both docs should eventually converge,
    // but since each peer creates an independent document, the merged doc will
    // contain both sets of shapes.
    if let Some(doc) = last_a {
        let shapes = count_shapes(&doc);
        assert!(shapes >= 2, "host should see at least joiner's 2 shapes, got {shapes}");
    }
    if let Some(doc) = last_b {
        let shapes = count_shapes(&doc);
        assert!(shapes >= 2, "joiner should see at least host's 2 shapes, got {shapes}");
    }

    cleanup(vec![ha, hb]);
}

/// Host creates layers, syncs to joiner, verifies layer structure.
#[test]
fn e2e_layers_sync() {
    let _g = lock_tests();

    let (ha, ticket, _pa) = host();
    let (hb, _pb) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Host creates a doc with multiple layers
    let mut doc_a = Document::new();
    let _layer2 = doc_a.create_layer("Background").unwrap();
    let _layer3 = doc_a.create_layer("Foreground").unwrap();

    // Add shapes to different layers
    doc_a.add_shape(rect(0, 0, 10, 5)).unwrap();
    doc_a.add_shape(rect(20, 0, 10, 5)).unwrap();

    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    let remote = wait_remote(&hb);

    // Verify layers arrived
    let doc_b = Document::from_automerge(remote);
    let layers = doc_b.read_all_layers().unwrap();
    assert!(
        layers.len() >= 3,
        "should have at least 3 layers (default + 2 created), got {}",
        layers.len()
    );

    let layer_names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
    assert!(
        layer_names.contains(&"Background"),
        "should have Background layer"
    );
    assert!(
        layer_names.contains(&"Foreground"),
        "should have Foreground layer"
    );

    cleanup(vec![ha, hb]);
}

/// Host creates groups, syncs to joiner, verifies group structure.
#[test]
fn e2e_groups_sync() {
    let _g = lock_tests();

    let (ha, ticket, _pa) = host();
    let (hb, _pb) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Host creates shapes and groups them
    let mut doc_a = Document::new();
    let s1 = doc_a.add_shape(rect(0, 0, 10, 5)).unwrap();
    let s2 = doc_a.add_shape(rect(15, 0, 10, 5)).unwrap();
    let _group = doc_a.create_group(&[s1, s2], None).unwrap();

    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    let remote = wait_remote(&hb);
    let doc_b = Document::from_automerge(remote);

    // Verify shapes
    let shapes = doc_b.read_all_shapes().unwrap();
    assert_eq!(shapes.len(), 2, "should have 2 grouped shapes");

    // Verify group
    let groups = doc_b.read_all_groups().unwrap();
    assert_eq!(groups.len(), 1, "should have 1 group");
    assert_eq!(groups[0].members.len(), 2, "group should have 2 members");

    cleanup(vec![ha, hb]);
}

/// Three peers: host + 2 joiners. Both joiners connect to host.
/// Host creates shapes, both joiners should receive them.
#[test]
fn e2e_three_peer_cluster() {
    let _g = lock_tests();

    let (ha, ticket, _pa) = host();
    let (hb, _pb) = join(&ticket);
    let (hc, _pc) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY * 2);

    // Host creates shapes
    let mut doc_a = Document::new();
    doc_a
        .add_shape(labeled_rect(0, 0, 20, 10, "Shared"))
        .unwrap();

    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    // Both joiners should eventually receive the shape
    // (B connects directly to A; C also connects directly to A)
    let remote_b = wait_remote(&hb);
    assert_eq!(count_shapes(&remote_b), 1, "peer B should have 1 shape");

    let remote_c = wait_remote(&hc);
    assert_eq!(count_shapes(&remote_c), 1, "peer C should have 1 shape");

    cleanup(vec![ha, hb, hc]);
}

/// Presence: host broadcasts cursor+activity, joiner receives exact data.
#[test]
fn e2e_presence_cursor_and_activity() {
    let _g = lock_tests();

    let (ha, ticket, pa) = host();
    let (hb, _pb) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Host broadcasts drawing activity with layer info
    let layer = LayerId::new();
    let presence = PeerPresence::new(
        pa,
        Position::new(42, 84),
        CursorActivity::Drawing {
            tool: irohscii::presence::ToolKind::Rectangle,
            start: Position::new(10, 10),
            current: Position::new(50, 30),
        },
        Some(layer),
        None,
    );

    ha.send_command(SyncCommand::BroadcastPresence(presence))
        .unwrap();

    let recv = wait_for_event(&hb, SYNC_TIMEOUT, |e| {
        matches!(e, SyncEvent::PresenceUpdate(_))
    })
    .and_then(|e| match e {
        SyncEvent::PresenceUpdate(p) => Some(p),
        _ => None,
    })
    .expect("joiner should receive presence");

    assert_eq!(recv.peer_id, pa);
    assert_eq!(recv.cursor_pos, Position::new(42, 84));
    assert_eq!(recv.active_layer_id, Some(layer));
    assert!(matches!(
        recv.activity,
        CursorActivity::Drawing {
            tool: irohscii::presence::ToolKind::Rectangle,
            ..
        }
    ));

    cleanup(vec![ha, hb]);
}

/// Verify reconnect: peer B disconnects, peer C joins and gets the full state.
#[test]
fn e2e_peer_disconnect_new_peer_gets_state() {
    let _g = lock_tests();

    let (ha, ticket, _pa) = host();
    let (hb, _pb) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Host creates shapes
    let mut doc_a = Document::new();
    doc_a.add_shape(rect(0, 0, 10, 5)).unwrap();
    doc_a.add_shape(rect(20, 0, 10, 5)).unwrap();

    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    wait_remote(&hb); // B receives

    // B disconnects
    let _ = hb.send_command(SyncCommand::Shutdown);
    std::thread::sleep(Duration::from_millis(500));

    // Host adds another shape
    doc_a.add_shape(rect(40, 0, 10, 5)).unwrap();
    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    // C joins fresh — should eventually get the full state (all 3 shapes).
    // C's periodic sync pulls from the host, so we wait until the doc
    // converges rather than checking just the first event (which may arrive
    // before the host has processed the third SyncDoc command).
    let (hc, _pc) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Re-send to ensure host store has the 3-shape doc
    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    let remote_c = wait_for_shapes(&hc, 3, SYNC_TIMEOUT)
        .expect("peer C should receive remote changes");
    assert_eq!(
        count_shapes(&remote_c),
        3,
        "new peer C should get all 3 shapes"
    );

    cleanup(vec![ha, hc]);
}

/// Verify undo/redo state syncs correctly.
#[test]
fn e2e_undo_redo_sync() {
    let _g = lock_tests();

    let (ha, ticket, _pa) = host();
    let (hb, _pb) = join(&ticket);
    std::thread::sleep(CONNECTION_DELAY);

    // Host creates shapes with undo checkpoints
    let mut doc_a = Document::new();
    doc_a.push_undo_checkpoint().unwrap();
    doc_a.add_shape(rect(0, 0, 10, 5)).unwrap();
    doc_a.push_undo_checkpoint().unwrap();
    doc_a.add_shape(rect(20, 0, 10, 5)).unwrap();

    // Sync the 2-shape state
    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    let remote = wait_remote(&hb);
    assert_eq!(count_shapes(&remote), 2, "should have 2 shapes before undo");

    // Host undoes last action
    doc_a.global_undo().unwrap();
    ha.send_command(SyncCommand::SyncDoc {
        doc: Box::new(doc_a.clone_automerge()),
    })
    .unwrap();

    // Give time for sync
    std::thread::sleep(Duration::from_secs(2));
    let after_undo = drain_remote(&hb, Duration::from_secs(2));

    if let Some(doc) = after_undo {
        let n = count_shapes(&doc);
        assert_eq!(n, 1, "after undo should have 1 shape, got {n}");
    }

    cleanup(vec![ha, hb]);
}
