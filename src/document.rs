//! Local-first automerge document - THE source of truth for all shape data.
//!
//! Every edit goes through this document. It handles:
//! - Shape storage and mutation
//! - Persistence to disk
//! - Sync with remote peers via automerge merge

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use automerge::{transaction::Transactable, Automerge, ObjId, ObjType, ReadDoc, ROOT};
use uuid::Uuid;

use crate::canvas::{LineStyle, Position};
use crate::layers::{Layer, LayerId};
use crate::shapes::{ShapeColor, ShapeKind};

/// Get the default storage path for the automerge document
pub fn default_storage_path() -> PathBuf {
    // Use XDG data directory if available, otherwise fallback to ~/.local/share
    let data_dir = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local/share")
        });
    data_dir.join("irohscii").join("document.automerge")
}

/// Unique identifier for a document (for sharing/sync)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Shape identifier - UUID for global uniqueness (CRDT-friendly)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ShapeId(pub Uuid);

impl ShapeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ShapeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ShapeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Group identifier - UUID for global uniqueness (CRDT-friendly)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GroupId(pub Uuid);

impl GroupId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for GroupId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Group data structure
#[derive(Debug, Clone)]
pub struct Group {
    pub id: GroupId,
    pub members: Vec<ShapeId>,
    pub parent: Option<GroupId>,
}

/// The automerge-backed document - THE source of truth
pub struct Document {
    /// The automerge document
    doc: Automerge,
    /// Unique document ID (for sync/persistence)
    id: DocumentId,
    /// Path where document is persisted (if any)
    storage_path: Option<PathBuf>,
    /// Whether there are unsaved changes
    dirty: bool,
}

impl Document {
    /// Create a new empty document
    pub fn new() -> Self {
        let id = DocumentId::new();
        let mut doc = Automerge::new();

        // Initialize document structure
        let default_layer_id = LayerId::new();
        {
            let mut tx = doc.transaction();
            tx.put(ROOT, "id", id.0.to_string()).unwrap();
            tx.put_object(ROOT, "shapes", ObjType::Map).unwrap();
            tx.put_object(ROOT, "shape_order", ObjType::List).unwrap();
            tx.put_object(ROOT, "groups", ObjType::Map).unwrap();

            // Initialize layers
            let layers_obj = tx.put_object(ROOT, "layers", ObjType::Map).unwrap();
            let layer_order_obj = tx.put_object(ROOT, "layer_order", ObjType::List).unwrap();

            // Create default layer
            let layer_obj = tx.put_object(&layers_obj, &default_layer_id.to_string(), ObjType::Map).unwrap();
            tx.put(&layer_obj, "name", "Layer 1").unwrap();
            tx.put(&layer_obj, "visible", true).unwrap();
            tx.put(&layer_obj, "locked", false).unwrap();
            tx.insert(&layer_order_obj, 0, default_layer_id.to_string()).unwrap();

            tx.put_object(ROOT, "undo_stack", ObjType::List).unwrap();
            tx.put_object(ROOT, "redo_stack", ObjType::List).unwrap();
            tx.commit();
        }

        Self {
            doc,
            id,
            storage_path: None,
            dirty: false,
        }
    }

    /// Create from an existing automerge document
    pub fn from_automerge(doc: Automerge, id: DocumentId) -> Self {
        Self {
            doc,
            id,
            storage_path: None,
            dirty: false,
        }
    }

    /// Load from disk
    pub fn load(path: &PathBuf) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let doc = Automerge::load(&bytes)?;

        // Extract document ID
        let id_str: String = doc
            .get(ROOT, "id")?
            .map(|(v, _)| v.to_string())
            .unwrap_or_default();
        let id = DocumentId(Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()));

        Ok(Self {
            doc,
            id,
            storage_path: Some(path.clone()),
            dirty: false,
        })
    }

    /// Save to disk
    pub fn save(&mut self) -> Result<()> {
        if let Some(path) = &self.storage_path {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let bytes = self.doc.save();
            std::fs::write(path, bytes)?;
            self.dirty = false;
        }
        Ok(())
    }

    /// Save to a specific path
    pub fn save_to(&mut self, path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = self.doc.save();
        std::fs::write(path, bytes)?;
        self.storage_path = Some(path.clone());
        self.dirty = false;
        Ok(())
    }

    /// Get the underlying automerge document (for sync)
    pub fn automerge(&self) -> &Automerge {
        &self.doc
    }

    /// Get mutable reference (for merging remote changes)
    pub fn automerge_mut(&mut self) -> &mut Automerge {
        &mut self.doc
    }

    /// Clone the automerge document (for sync)
    pub fn clone_automerge(&self) -> Automerge {
        self.doc.clone()
    }

    /// Merge remote changes
    pub fn merge(&mut self, other: &mut Automerge) -> Result<()> {
        self.doc.merge(other)?;
        self.dirty = true;
        Ok(())
    }

    /// Get document ID
    pub fn id(&self) -> DocumentId {
        self.id
    }

    /// Check if dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get storage path
    pub fn storage_path(&self) -> Option<&PathBuf> {
        self.storage_path.as_ref()
    }

    /// Set storage path
    pub fn set_storage_path(&mut self, path: PathBuf) {
        self.storage_path = Some(path);
    }

    // --- Shape Operations ---

    fn get_shapes_map(&self) -> Result<ObjId> {
        match self.doc.get(ROOT, "shapes")? {
            Some((_, obj_id)) => Ok(obj_id),
            None => Err(anyhow!("No shapes map in document")),
        }
    }

    fn ensure_shapes_map(&mut self) -> Result<ObjId> {
        match self.doc.get(ROOT, "shapes")? {
            Some((_, obj_id)) => Ok(obj_id),
            None => {
                // Create the shapes map
                let mut tx = self.doc.transaction();
                tx.put_object(ROOT, "shapes", ObjType::Map)?;
                tx.commit();
                // Re-fetch the ObjId after commit (the one from tx is stale)
                match self.doc.get(ROOT, "shapes")? {
                    Some((_, obj_id)) => Ok(obj_id),
                    None => Err(anyhow!("Failed to create shapes map")),
                }
            }
        }
    }

    fn get_shape_order_list(&self) -> Result<ObjId> {
        match self.doc.get(ROOT, "shape_order")? {
            Some((_, obj_id)) => Ok(obj_id),
            None => Err(anyhow!("No shape_order list in document")),
        }
    }

    /// Add a new shape to the document
    pub fn add_shape(&mut self, kind: ShapeKind) -> Result<ShapeId> {
        let id = ShapeId::new();

        let mut tx = self.doc.transaction();

        // Get or create shapes map within this transaction
        let shapes_obj = match tx.get(ROOT, "shapes")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "shapes", ObjType::Map)?,
        };

        let shape_obj = tx.put_object(&shapes_obj, &id.to_string(), ObjType::Map)?;
        write_shape_kind(&mut tx, &shape_obj, &kind)?;

        // Append to shape_order list (new shapes go on top)
        let order_obj = match tx.get(ROOT, "shape_order")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "shape_order", ObjType::List)?,
        };
        let len = tx.length(&order_obj);
        tx.insert(&order_obj, len, id.to_string())?;

        tx.commit();

        self.dirty = true;
        Ok(id)
    }

    /// Update an existing shape
    pub fn update_shape(&mut self, id: ShapeId, kind: ShapeKind) -> Result<()> {
        let shapes_obj = self.get_shapes_map()?;

        // Delete old shape object and create new one with updated data
        let mut tx = self.doc.transaction();
        tx.delete(&shapes_obj, &id.to_string())?;
        let shape_obj = tx.put_object(&shapes_obj, &id.to_string(), ObjType::Map)?;
        write_shape_kind(&mut tx, &shape_obj, &kind)?;
        tx.commit();

        self.dirty = true;
        Ok(())
    }

    /// Delete a shape
    pub fn delete_shape(&mut self, id: ShapeId) -> Result<()> {
        let shapes_obj = self.get_shapes_map()?;

        let mut tx = self.doc.transaction();
        tx.delete(&shapes_obj, &id.to_string())?;

        // Remove from shape_order list
        if let Some((_, order_obj)) = tx.get(ROOT, "shape_order")? {
            let id_str = id.to_string();
            let len = tx.length(&order_obj);
            for i in (0..len).rev() {
                if let Some((automerge::Value::Scalar(s), _)) = tx.get(&order_obj, i)? {
                    if s.to_string().trim_matches('"') == id_str {
                        tx.delete(&order_obj, i)?;
                        break;
                    }
                }
            }
        }

        tx.commit();

        self.dirty = true;
        Ok(())
    }

    /// Read a single shape
    pub fn read_shape(&self, id: ShapeId) -> Result<Option<ShapeKind>> {
        let shapes_obj = self.get_shapes_map()?;
        if let Some((_, shape_obj)) = self.doc.get(&shapes_obj, &id.to_string())? {
            read_shape_kind(&self.doc, &shape_obj)
        } else {
            Ok(None)
        }
    }

    /// Read all shapes (for building cache)
    pub fn read_all_shapes(&self) -> Result<Vec<(ShapeId, ShapeKind)>> {
        let shapes_obj = match self.doc.get(ROOT, "shapes")? {
            Some((_, obj_id)) => obj_id,
            None => return Ok(Vec::new()),
        };

        let mut shapes = Vec::new();

        for key in self.doc.keys(&shapes_obj) {
            let id = ShapeId(Uuid::parse_str(&key)?);
            if let Some((_, shape_obj)) = self.doc.get(&shapes_obj, &key)? {
                if let Some(kind) = read_shape_kind(&self.doc, &shape_obj)? {
                    shapes.push((id, kind));
                }
            }
        }
        Ok(shapes)
    }

    // --- Z-Order Operations ---

    /// Read the shape order list (returns ShapeIds in render order, bottom to top)
    pub fn read_shape_order(&self) -> Result<Vec<ShapeId>> {
        let order_obj = match self.doc.get(ROOT, "shape_order")? {
            Some((_, obj_id)) => obj_id,
            None => return Ok(Vec::new()),
        };

        let len = self.doc.length(&order_obj);
        let mut order = Vec::with_capacity(len);

        for i in 0..len {
            if let Some((automerge::Value::Scalar(s), _)) = self.doc.get(&order_obj, i)? {
                let id_str = s.to_string().trim_matches('"').to_string();
                if let Ok(uuid) = Uuid::parse_str(&id_str) {
                    order.push(ShapeId(uuid));
                }
            }
        }

        Ok(order)
    }

    /// Bring shapes to front (move to end of order list)
    pub fn bring_to_front(&mut self, ids: &[ShapeId]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut current_order = self.read_shape_order()?;
        let id_set: std::collections::HashSet<_> = ids.iter().collect();

        // Remove the shapes from their current positions
        let mut moved: Vec<ShapeId> = Vec::new();
        current_order.retain(|id| {
            if id_set.contains(id) {
                moved.push(*id);
                false
            } else {
                true
            }
        });

        // Append them at the end (top)
        current_order.extend(moved);

        self.set_shape_order(&current_order)
    }

    /// Send shapes to back (move to start of order list)
    pub fn send_to_back(&mut self, ids: &[ShapeId]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut current_order = self.read_shape_order()?;
        let id_set: std::collections::HashSet<_> = ids.iter().collect();

        // Remove the shapes from their current positions
        let mut moved: Vec<ShapeId> = Vec::new();
        current_order.retain(|id| {
            if id_set.contains(id) {
                moved.push(*id);
                false
            } else {
                true
            }
        });

        // Prepend them at the start (bottom)
        moved.extend(current_order);

        self.set_shape_order(&moved)
    }

    /// Bring shapes forward one position (toward top)
    pub fn bring_forward(&mut self, ids: &[ShapeId]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut order = self.read_shape_order()?;
        let id_set: std::collections::HashSet<_> = ids.iter().collect();

        // Work from end to start to avoid cascading swaps
        for i in (0..order.len().saturating_sub(1)).rev() {
            if id_set.contains(&order[i]) && !id_set.contains(&order[i + 1]) {
                order.swap(i, i + 1);
            }
        }

        self.set_shape_order(&order)
    }

    /// Send shapes backward one position (toward bottom)
    pub fn send_backward(&mut self, ids: &[ShapeId]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut order = self.read_shape_order()?;
        let id_set: std::collections::HashSet<_> = ids.iter().collect();

        // Work from start to end to avoid cascading swaps
        for i in 1..order.len() {
            if id_set.contains(&order[i]) && !id_set.contains(&order[i - 1]) {
                order.swap(i, i - 1);
            }
        }

        self.set_shape_order(&order)
    }

    /// Set the complete shape order (internal helper)
    fn set_shape_order(&mut self, order: &[ShapeId]) -> Result<()> {
        let mut tx = self.doc.transaction();

        // Get or create order list
        let order_obj = match tx.get(ROOT, "shape_order")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "shape_order", ObjType::List)?,
        };

        // Clear existing entries
        let len = tx.length(&order_obj);
        for i in (0..len).rev() {
            tx.delete(&order_obj, i)?;
        }

        // Insert new order
        for (i, id) in order.iter().enumerate() {
            tx.insert(&order_obj, i, id.to_string())?;
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    // --- Group Operations ---

    fn get_groups_map(&self) -> Result<ObjId> {
        match self.doc.get(ROOT, "groups")? {
            Some((_, obj_id)) => Ok(obj_id),
            None => Err(anyhow!("No groups map in document")),
        }
    }

    /// Create a group from a set of shapes
    pub fn create_group(&mut self, members: &[ShapeId], parent: Option<GroupId>) -> Result<GroupId> {
        if members.is_empty() {
            return Err(anyhow!("Cannot create empty group"));
        }

        let id = GroupId::new();

        let mut tx = self.doc.transaction();

        // Get or create groups map
        let groups_obj = match tx.get(ROOT, "groups")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "groups", ObjType::Map)?,
        };

        // Create the group object
        let group_obj = tx.put_object(&groups_obj, &id.to_string(), ObjType::Map)?;

        // Add members list
        let members_obj = tx.put_object(&group_obj, "members", ObjType::List)?;
        for (i, member_id) in members.iter().enumerate() {
            tx.insert(&members_obj, i, member_id.to_string())?;
        }

        // Add parent if specified
        if let Some(parent_id) = parent {
            tx.put(&group_obj, "parent", parent_id.to_string())?;
        }

        tx.commit();

        // Update shapes to reference this group
        for member_id in members {
            self.set_shape_group(*member_id, Some(id))?;
        }

        self.dirty = true;
        Ok(id)
    }

    /// Delete a group (ungroups all shapes)
    pub fn delete_group(&mut self, id: GroupId) -> Result<()> {
        // First, clear group reference from all member shapes
        if let Some(group) = self.read_group(id)? {
            for member_id in group.members {
                let _ = self.set_shape_group(member_id, None);
            }
        }

        // Remove the group from the groups map
        let groups_obj = self.get_groups_map()?;

        let mut tx = self.doc.transaction();
        tx.delete(&groups_obj, &id.to_string())?;
        tx.commit();

        self.dirty = true;
        Ok(())
    }

    /// Read a single group
    pub fn read_group(&self, id: GroupId) -> Result<Option<Group>> {
        let groups_obj = match self.doc.get(ROOT, "groups")? {
            Some((_, obj_id)) => obj_id,
            None => return Ok(None),
        };

        match self.doc.get(&groups_obj, &id.to_string())? {
            Some((_, group_obj)) => {
                // Read members
                let members = match self.doc.get(&group_obj, "members")? {
                    Some((_, members_obj)) => {
                        let len = self.doc.length(&members_obj);
                        let mut member_ids = Vec::with_capacity(len);
                        for i in 0..len {
                            if let Some((automerge::Value::Scalar(s), _)) = self.doc.get(&members_obj, i)? {
                                let id_str = s.to_string().trim_matches('"').to_string();
                                if let Ok(uuid) = Uuid::parse_str(&id_str) {
                                    member_ids.push(ShapeId(uuid));
                                }
                            }
                        }
                        member_ids
                    }
                    None => Vec::new(),
                };

                // Read parent
                let parent = match self.doc.get(&group_obj, "parent")? {
                    Some((automerge::Value::Scalar(s), _)) => {
                        let id_str = s.to_string().trim_matches('"').to_string();
                        Uuid::parse_str(&id_str).ok().map(GroupId)
                    }
                    _ => None,
                };

                Ok(Some(Group { id, members, parent }))
            }
            None => Ok(None),
        }
    }

    /// Read all groups
    pub fn read_all_groups(&self) -> Result<Vec<Group>> {
        let groups_obj = match self.doc.get(ROOT, "groups")? {
            Some((_, obj_id)) => obj_id,
            None => return Ok(Vec::new()),
        };

        let mut groups = Vec::new();
        for key in self.doc.keys(&groups_obj) {
            if let Ok(uuid) = Uuid::parse_str(&key) {
                let id = GroupId(uuid);
                if let Some(group) = self.read_group(id)? {
                    groups.push(group);
                }
            }
        }
        Ok(groups)
    }

    /// Get the group a shape belongs to (reads from shape's group_id field)
    pub fn get_shape_group(&self, id: ShapeId) -> Result<Option<GroupId>> {
        let shapes_obj = self.get_shapes_map()?;

        match self.doc.get(&shapes_obj, &id.to_string())? {
            Some((_, shape_obj)) => {
                match self.doc.get(&shape_obj, "group_id")? {
                    Some((automerge::Value::Scalar(s), _)) => {
                        let id_str = s.to_string().trim_matches('"').to_string();
                        Ok(Uuid::parse_str(&id_str).ok().map(GroupId))
                    }
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    /// Set or clear the group a shape belongs to
    pub fn set_shape_group(&mut self, id: ShapeId, group_id: Option<GroupId>) -> Result<()> {
        let shapes_obj = self.get_shapes_map()?;

        let mut tx = self.doc.transaction();

        if let Some((_, shape_obj)) = tx.get(&shapes_obj, &id.to_string())? {
            match group_id {
                Some(gid) => {
                    tx.put(&shape_obj, "group_id", gid.to_string())?;
                }
                None => {
                    // Try to delete the group_id field if it exists
                    let _ = tx.delete(&shape_obj, "group_id");
                }
            }
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    /// Get the root group of a group (traverse up the parent chain)
    pub fn get_root_group(&self, id: GroupId) -> Result<GroupId> {
        let mut current = id;
        let mut seen = std::collections::HashSet::new();

        while let Some(group) = self.read_group(current)? {
            if let Some(parent) = group.parent {
                if seen.contains(&parent) {
                    // Circular reference, break
                    break;
                }
                seen.insert(current);
                current = parent;
            } else {
                break;
            }
        }

        Ok(current)
    }

    /// Get all shapes in a group (including nested groups)
    pub fn get_all_group_shapes(&self, id: GroupId) -> Result<Vec<ShapeId>> {
        let mut all_shapes = Vec::new();
        let mut to_visit = vec![id];
        let mut visited = std::collections::HashSet::new();

        while let Some(group_id) = to_visit.pop() {
            if visited.contains(&group_id) {
                continue;
            }
            visited.insert(group_id);

            if let Some(group) = self.read_group(group_id)? {
                all_shapes.extend(group.members.clone());

                // Find nested groups
                for group in self.read_all_groups()? {
                    if group.parent == Some(group_id) {
                        to_visit.push(group.id);
                    }
                }
            }
        }

        Ok(all_shapes)
    }

    // --- Layer Operations ---

    fn get_layers_map(&self) -> Result<ObjId> {
        match self.doc.get(ROOT, "layers")? {
            Some((_, obj_id)) => Ok(obj_id),
            None => Err(anyhow!("No layers map in document")),
        }
    }

    /// Get the layer order list
    pub fn read_layer_order(&self) -> Result<Vec<LayerId>> {
        let order_obj = match self.doc.get(ROOT, "layer_order")? {
            Some((_, obj_id)) => obj_id,
            None => return Ok(Vec::new()),
        };

        let len = self.doc.length(&order_obj);
        let mut order = Vec::with_capacity(len);

        for i in 0..len {
            if let Some((automerge::Value::Scalar(s), _)) = self.doc.get(&order_obj, i)? {
                let id_str = s.to_string().trim_matches('"').to_string();
                if let Ok(uuid) = Uuid::parse_str(&id_str) {
                    order.push(LayerId(uuid));
                }
            }
        }

        Ok(order)
    }

    /// Get the default (first) layer ID
    pub fn get_default_layer(&self) -> Result<LayerId> {
        let order = self.read_layer_order()?;
        order.first().copied().ok_or_else(|| anyhow!("No layers in document"))
    }

    /// Create a new layer
    pub fn create_layer(&mut self, name: &str) -> Result<LayerId> {
        let id = LayerId::new();

        let mut tx = self.doc.transaction();

        // Get or create layers map
        let layers_obj = match tx.get(ROOT, "layers")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "layers", ObjType::Map)?,
        };

        // Create the layer object
        let layer_obj = tx.put_object(&layers_obj, &id.to_string(), ObjType::Map)?;
        tx.put(&layer_obj, "name", name)?;
        tx.put(&layer_obj, "visible", true)?;
        tx.put(&layer_obj, "locked", false)?;

        // Add to layer order (at the top)
        let order_obj = match tx.get(ROOT, "layer_order")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "layer_order", ObjType::List)?,
        };
        let len = tx.length(&order_obj);
        tx.insert(&order_obj, len, id.to_string())?;

        tx.commit();
        self.dirty = true;
        Ok(id)
    }

    /// Delete a layer (moves shapes to default layer)
    pub fn delete_layer(&mut self, id: LayerId) -> Result<()> {
        let layer_order = self.read_layer_order()?;

        // Can't delete the last layer
        if layer_order.len() <= 1 {
            return Err(anyhow!("Cannot delete the last layer"));
        }

        // Find the default layer to move shapes to
        let default_layer = layer_order.iter()
            .find(|&&lid| lid != id)
            .copied()
            .ok_or_else(|| anyhow!("No other layer to move shapes to"))?;

        // Move all shapes on this layer to the default layer
        let all_shapes = self.read_all_shapes()?;
        for (shape_id, _) in &all_shapes {
            if let Ok(Some(layer_id)) = self.get_shape_layer(*shape_id) {
                if layer_id == id {
                    self.set_shape_layer(*shape_id, default_layer)?;
                }
            }
        }

        // Remove the layer
        let layers_obj = self.get_layers_map()?;

        let mut tx = self.doc.transaction();
        tx.delete(&layers_obj, &id.to_string())?;

        // Remove from layer order
        if let Some((_, order_obj)) = tx.get(ROOT, "layer_order")? {
            let id_str = id.to_string();
            let len = tx.length(&order_obj);
            for i in (0..len).rev() {
                if let Some((automerge::Value::Scalar(s), _)) = tx.get(&order_obj, i)? {
                    if s.to_string().trim_matches('"') == id_str {
                        tx.delete(&order_obj, i)?;
                        break;
                    }
                }
            }
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    /// Read a single layer
    pub fn read_layer(&self, id: LayerId) -> Result<Option<Layer>> {
        let layers_obj = match self.doc.get(ROOT, "layers")? {
            Some((_, obj_id)) => obj_id,
            None => return Ok(None),
        };

        match self.doc.get(&layers_obj, &id.to_string())? {
            Some((_, layer_obj)) => {
                let name = match self.doc.get(&layer_obj, "name")? {
                    Some((automerge::Value::Scalar(s), _)) => {
                        s.to_string().trim_matches('"').to_string()
                    }
                    _ => "Unnamed".to_string(),
                };

                let visible = match self.doc.get(&layer_obj, "visible")? {
                    Some((automerge::Value::Scalar(s), _)) => {
                        s.to_bool().unwrap_or(true)
                    }
                    _ => true,
                };

                let locked = match self.doc.get(&layer_obj, "locked")? {
                    Some((automerge::Value::Scalar(s), _)) => {
                        s.to_bool().unwrap_or(false)
                    }
                    _ => false,
                };

                Ok(Some(Layer { id, name, visible, locked }))
            }
            None => Ok(None),
        }
    }

    /// Read all layers (in order)
    pub fn read_all_layers(&self) -> Result<Vec<Layer>> {
        let order = self.read_layer_order()?;
        let mut layers = Vec::with_capacity(order.len());

        for id in order {
            if let Some(layer) = self.read_layer(id)? {
                layers.push(layer);
            }
        }

        Ok(layers)
    }

    /// Rename a layer
    pub fn rename_layer(&mut self, id: LayerId, name: &str) -> Result<()> {
        let layers_obj = self.get_layers_map()?;

        let mut tx = self.doc.transaction();

        if let Some((_, layer_obj)) = tx.get(&layers_obj, &id.to_string())? {
            tx.put(&layer_obj, "name", name)?;
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    /// Set layer visibility
    pub fn set_layer_visible(&mut self, id: LayerId, visible: bool) -> Result<()> {
        let layers_obj = self.get_layers_map()?;

        let mut tx = self.doc.transaction();

        if let Some((_, layer_obj)) = tx.get(&layers_obj, &id.to_string())? {
            tx.put(&layer_obj, "visible", visible)?;
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    /// Set layer locked state
    pub fn set_layer_locked(&mut self, id: LayerId, locked: bool) -> Result<()> {
        let layers_obj = self.get_layers_map()?;

        let mut tx = self.doc.transaction();

        if let Some((_, layer_obj)) = tx.get(&layers_obj, &id.to_string())? {
            tx.put(&layer_obj, "locked", locked)?;
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    /// Get the layer a shape belongs to
    pub fn get_shape_layer(&self, id: ShapeId) -> Result<Option<LayerId>> {
        let shapes_obj = self.get_shapes_map()?;

        match self.doc.get(&shapes_obj, &id.to_string())? {
            Some((_, shape_obj)) => {
                match self.doc.get(&shape_obj, "layer_id")? {
                    Some((automerge::Value::Scalar(s), _)) => {
                        let id_str = s.to_string().trim_matches('"').to_string();
                        Ok(Uuid::parse_str(&id_str).ok().map(LayerId))
                    }
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    /// Set the layer a shape belongs to
    pub fn set_shape_layer(&mut self, id: ShapeId, layer_id: LayerId) -> Result<()> {
        let shapes_obj = self.get_shapes_map()?;

        let mut tx = self.doc.transaction();

        if let Some((_, shape_obj)) = tx.get(&shapes_obj, &id.to_string())? {
            tx.put(&shape_obj, "layer_id", layer_id.to_string())?;
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    /// Move layer in the order (toward top = higher index)
    pub fn move_layer(&mut self, id: LayerId, new_index: usize) -> Result<()> {
        let mut order = self.read_layer_order()?;

        // Find current position
        let current_pos = order.iter().position(|&lid| lid == id)
            .ok_or_else(|| anyhow!("Layer not found in order"))?;

        // Remove from current position
        order.remove(current_pos);

        // Insert at new position (clamped to valid range)
        let new_pos = new_index.min(order.len());
        order.insert(new_pos, id);

        // Save the new order
        let mut tx = self.doc.transaction();

        let order_obj = match tx.get(ROOT, "layer_order")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "layer_order", ObjType::List)?,
        };

        // Clear existing entries
        let len = tx.length(&order_obj);
        for i in (0..len).rev() {
            tx.delete(&order_obj, i)?;
        }

        // Insert new order
        for (i, layer_id) in order.iter().enumerate() {
            tx.insert(&order_obj, i, layer_id.to_string())?;
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    /// Translate a shape by delta
    pub fn translate_shape(&mut self, id: ShapeId, dx: i32, dy: i32) -> Result<()> {
        if let Some(kind) = self.read_shape(id)? {
            let translated = kind.translated(dx, dy);
            self.update_shape(id, translated)?;
        }
        Ok(())
    }

    /// Update connected lines when a shape moves
    /// Returns the IDs of shapes that were modified
    pub fn update_connections_for_shape(&mut self, moved_id: ShapeId, dx: i32, dy: i32) -> Result<Vec<ShapeId>> {
        let all_shapes = self.read_all_shapes()?;
        let mut updated = Vec::new();

        for (id, kind) in all_shapes {
            match kind {
                ShapeKind::Line { start, end, style, start_connection, end_connection, label, color } => {
                    let mut changed = false;
                    let mut new_start = start;
                    let mut new_end = end;

                    if start_connection == Some(moved_id.0.as_u128() as u64) {
                        new_start = Position::new(start.x + dx, start.y + dy);
                        changed = true;
                    }
                    if end_connection == Some(moved_id.0.as_u128() as u64) {
                        new_end = Position::new(end.x + dx, end.y + dy);
                        changed = true;
                    }

                    if changed {
                        self.update_shape(id, ShapeKind::Line {
                            start: new_start,
                            end: new_end,
                            style,
                            start_connection,
                            end_connection,
                            label,
                            color,
                        })?;
                        updated.push(id);
                    }
                }
                ShapeKind::Arrow { start, end, style, start_connection, end_connection, label, color } => {
                    let mut changed = false;
                    let mut new_start = start;
                    let mut new_end = end;

                    if start_connection == Some(moved_id.0.as_u128() as u64) {
                        new_start = Position::new(start.x + dx, start.y + dy);
                        changed = true;
                    }
                    if end_connection == Some(moved_id.0.as_u128() as u64) {
                        new_end = Position::new(end.x + dx, end.y + dy);
                        changed = true;
                    }

                    if changed {
                        self.update_shape(id, ShapeKind::Arrow {
                            start: new_start,
                            end: new_end,
                            style,
                            start_connection,
                            end_connection,
                            label,
                            color,
                        })?;
                        updated.push(id);
                    }
                }
                _ => {}
            }
        }
        Ok(updated)
    }

    /// Update connected lines when a shape is resized
    /// This handles the case where different snap points move by different amounts
    /// Returns the IDs of shapes that were modified
    pub fn update_connections_for_resize(&mut self, resized_id: ShapeId, old_kind: &ShapeKind, new_kind: &ShapeKind) -> Result<Vec<ShapeId>> {
        let old_snaps = old_kind.snap_points();
        let new_snaps = new_kind.snap_points();

        // If snap point counts don't match, we can't reliably update connections
        if old_snaps.len() != new_snaps.len() {
            return Ok(Vec::new());
        }

        let all_shapes = self.read_all_shapes()?;
        let resized_conn_id = resized_id.0.as_u128() as u64;
        let mut updated = Vec::new();

        for (id, kind) in all_shapes {
            match kind {
                ShapeKind::Line { start, end, style, start_connection, end_connection, label, color } => {
                    let mut changed = false;
                    let mut new_start = start;
                    let mut new_end = end;

                    // If start is connected to the resized shape, find which snap point it was at
                    if start_connection == Some(resized_conn_id) {
                        if let Some(new_pos) = find_corresponding_snap(&start, &old_snaps, &new_snaps) {
                            new_start = new_pos;
                            changed = true;
                        }
                    }

                    // If end is connected to the resized shape, find which snap point it was at
                    if end_connection == Some(resized_conn_id) {
                        if let Some(new_pos) = find_corresponding_snap(&end, &old_snaps, &new_snaps) {
                            new_end = new_pos;
                            changed = true;
                        }
                    }

                    if changed {
                        self.update_shape(id, ShapeKind::Line {
                            start: new_start,
                            end: new_end,
                            style,
                            start_connection,
                            end_connection,
                            label,
                            color,
                        })?;
                        updated.push(id);
                    }
                }
                ShapeKind::Arrow { start, end, style, start_connection, end_connection, label, color } => {
                    let mut changed = false;
                    let mut new_start = start;
                    let mut new_end = end;

                    if start_connection == Some(resized_conn_id) {
                        if let Some(new_pos) = find_corresponding_snap(&start, &old_snaps, &new_snaps) {
                            new_start = new_pos;
                            changed = true;
                        }
                    }

                    if end_connection == Some(resized_conn_id) {
                        if let Some(new_pos) = find_corresponding_snap(&end, &old_snaps, &new_snaps) {
                            new_end = new_pos;
                            changed = true;
                        }
                    }

                    if changed {
                        self.update_shape(id, ShapeKind::Arrow {
                            start: new_start,
                            end: new_end,
                            style,
                            start_connection,
                            end_connection,
                            label,
                            color,
                        })?;
                        updated.push(id);
                    }
                }
                _ => {}
            }
        }
        Ok(updated)
    }

    // --- Global Undo/Redo (synced via CRDT) ---

    /// Maximum undo history size
    const MAX_UNDO_HISTORY: usize = 50;

    /// Serialize current shapes state for undo checkpoint
    fn serialize_shapes(&self) -> Result<Vec<u8>> {
        let shapes = self.read_all_shapes()?;
        let bytes = rmp_serde::to_vec(&shapes)?;
        Ok(bytes)
    }

    /// Get the undo_stack list object
    fn get_undo_stack(&self) -> Result<ObjId> {
        match self.doc.get(ROOT, "undo_stack")? {
            Some((_, obj_id)) => Ok(obj_id),
            None => Err(anyhow!("No undo_stack in document")),
        }
    }

    /// Get the redo_stack list object
    fn get_redo_stack(&self) -> Result<ObjId> {
        match self.doc.get(ROOT, "redo_stack")? {
            Some((_, obj_id)) => Ok(obj_id),
            None => Err(anyhow!("No redo_stack in document")),
        }
    }

    /// Push current shapes state to undo stack (call before mutations)
    pub fn push_undo_checkpoint(&mut self) -> Result<()> {
        let snapshot = self.serialize_shapes()?;

        let mut tx = self.doc.transaction();

        // Get or create undo_stack
        let undo_stack = match tx.get(ROOT, "undo_stack")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "undo_stack", ObjType::List)?,
        };

        // Get or create redo_stack (to clear it)
        let redo_stack = match tx.get(ROOT, "redo_stack")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "redo_stack", ObjType::List)?,
        };

        // Clear redo stack on new action
        let redo_len = tx.length(&redo_stack);
        for i in (0..redo_len).rev() {
            tx.delete(&redo_stack, i)?;
        }

        // Push snapshot to undo stack
        let len = tx.length(&undo_stack);
        tx.insert(&undo_stack, len, snapshot)?;

        // Trim to max history
        while tx.length(&undo_stack) > Self::MAX_UNDO_HISTORY {
            tx.delete(&undo_stack, 0)?;
        }

        tx.commit();
        self.dirty = true;
        Ok(())
    }

    /// Undo: pop from undo stack, push current to redo, restore shapes
    pub fn global_undo(&mut self) -> Result<bool> {
        let undo_stack = self.get_undo_stack()?;
        let undo_len = self.doc.length(&undo_stack);

        if undo_len == 0 {
            return Ok(false);
        }

        // Get the last undo snapshot
        let prev_snapshot: Vec<u8> = match self.doc.get(&undo_stack, undo_len - 1)? {
            Some((automerge::Value::Scalar(s), _)) => {
                s.to_bytes().map(|b| b.to_vec()).ok_or_else(|| anyhow!("Invalid undo snapshot"))?
            }
            _ => return Err(anyhow!("Invalid undo snapshot")),
        };

        // Deserialize the previous shapes
        let prev_shapes: Vec<(ShapeId, ShapeKind)> = rmp_serde::from_slice(&prev_snapshot)?;

        // Save current state to redo stack
        let current_snapshot = self.serialize_shapes()?;

        // Collect existing shape keys before starting transaction
        let shapes_obj_for_keys = self.get_shapes_map()?;
        let existing_keys: Vec<String> = self.doc.keys(&shapes_obj_for_keys).collect();

        let mut tx = self.doc.transaction();

        // Pop from undo stack
        let undo_stack = match tx.get(ROOT, "undo_stack")? {
            Some((_, obj_id)) => obj_id,
            None => return Err(anyhow!("No undo_stack")),
        };
        let undo_len = tx.length(&undo_stack);
        tx.delete(&undo_stack, undo_len - 1)?;

        // Push to redo stack
        let redo_stack = match tx.get(ROOT, "redo_stack")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "redo_stack", ObjType::List)?,
        };
        let redo_len = tx.length(&redo_stack);
        tx.insert(&redo_stack, redo_len, current_snapshot)?;

        // Clear current shapes
        let shapes_obj = match tx.get(ROOT, "shapes")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "shapes", ObjType::Map)?,
        };
        for key in existing_keys {
            tx.delete(&shapes_obj, &key)?;
        }

        // Restore previous shapes
        for (id, kind) in prev_shapes {
            let shape_obj = tx.put_object(&shapes_obj, &id.to_string(), ObjType::Map)?;
            write_shape_kind(&mut tx, &shape_obj, &kind)?;
        }

        tx.commit();
        self.dirty = true;
        Ok(true)
    }

    /// Redo: pop from redo stack, push current to undo, restore shapes
    pub fn global_redo(&mut self) -> Result<bool> {
        let redo_stack = self.get_redo_stack()?;
        let redo_len = self.doc.length(&redo_stack);

        if redo_len == 0 {
            return Ok(false);
        }

        // Get the last redo snapshot
        let next_snapshot: Vec<u8> = match self.doc.get(&redo_stack, redo_len - 1)? {
            Some((automerge::Value::Scalar(s), _)) => {
                s.to_bytes().map(|b| b.to_vec()).ok_or_else(|| anyhow!("Invalid redo snapshot"))?
            }
            _ => return Err(anyhow!("Invalid redo snapshot")),
        };

        // Deserialize the next shapes
        let next_shapes: Vec<(ShapeId, ShapeKind)> = rmp_serde::from_slice(&next_snapshot)?;

        // Save current state to undo stack
        let current_snapshot = self.serialize_shapes()?;

        // Collect existing shape keys before starting transaction
        let shapes_obj_for_keys = self.get_shapes_map()?;
        let existing_keys: Vec<String> = self.doc.keys(&shapes_obj_for_keys).collect();

        let mut tx = self.doc.transaction();

        // Pop from redo stack
        let redo_stack = match tx.get(ROOT, "redo_stack")? {
            Some((_, obj_id)) => obj_id,
            None => return Err(anyhow!("No redo_stack")),
        };
        let redo_len = tx.length(&redo_stack);
        tx.delete(&redo_stack, redo_len - 1)?;

        // Push to undo stack
        let undo_stack = match tx.get(ROOT, "undo_stack")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "undo_stack", ObjType::List)?,
        };
        let undo_len = tx.length(&undo_stack);
        tx.insert(&undo_stack, undo_len, current_snapshot)?;

        // Clear current shapes
        let shapes_obj = match tx.get(ROOT, "shapes")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "shapes", ObjType::Map)?,
        };
        for key in existing_keys {
            tx.delete(&shapes_obj, &key)?;
        }

        // Restore next shapes
        for (id, kind) in next_shapes {
            let shape_obj = tx.put_object(&shapes_obj, &id.to_string(), ObjType::Map)?;
            write_shape_kind(&mut tx, &shape_obj, &kind)?;
        }

        tx.commit();
        self.dirty = true;
        Ok(true)
    }

    /// Clear undo/redo stacks (e.g., on new document)
    pub fn clear_undo_history(&mut self) -> Result<()> {
        let mut tx = self.doc.transaction();

        // Clear undo stack
        let undo_stack = match tx.get(ROOT, "undo_stack")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "undo_stack", ObjType::List)?,
        };
        let undo_len = tx.length(&undo_stack);
        for i in (0..undo_len).rev() {
            tx.delete(&undo_stack, i)?;
        }

        // Clear redo stack
        let redo_stack = match tx.get(ROOT, "redo_stack")? {
            Some((_, obj_id)) => obj_id,
            None => tx.put_object(ROOT, "redo_stack", ObjType::List)?,
        };
        let redo_len = tx.length(&redo_stack);
        for i in (0..redo_len).rev() {
            tx.delete(&redo_stack, i)?;
        }

        tx.commit();
        Ok(())
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

// --- Serialization helpers ---

fn write_shape_kind<T: Transactable>(tx: &mut T, obj: &ObjId, kind: &ShapeKind) -> Result<()> {
    match kind {
        ShapeKind::Line {
            start,
            end,
            style,
            start_connection,
            end_connection,
            label,
            color,
        } => {
            tx.put(obj, "kind", "Line")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "style", line_style_to_str(*style))?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(conn) = start_connection {
                tx.put(obj, "start_conn", *conn as i64)?;
            }
            if let Some(conn) = end_connection {
                tx.put(obj, "end_conn", *conn as i64)?;
            }
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Arrow {
            start,
            end,
            style,
            start_connection,
            end_connection,
            label,
            color,
        } => {
            tx.put(obj, "kind", "Arrow")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "style", line_style_to_str(*style))?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(conn) = start_connection {
                tx.put(obj, "start_conn", *conn as i64)?;
            }
            if let Some(conn) = end_connection {
                tx.put(obj, "end_conn", *conn as i64)?;
            }
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Rectangle { start, end, label, color } => {
            tx.put(obj, "kind", "Rectangle")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::DoubleBox { start, end, label, color } => {
            tx.put(obj, "kind", "DoubleBox")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Diamond {
            center,
            half_width,
            half_height,
            label,
            color,
        } => {
            tx.put(obj, "kind", "Diamond")?;
            tx.put(obj, "center_x", center.x as i64)?;
            tx.put(obj, "center_y", center.y as i64)?;
            tx.put(obj, "half_width", *half_width as i64)?;
            tx.put(obj, "half_height", *half_height as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Ellipse {
            center,
            radius_x,
            radius_y,
            label,
            color,
        } => {
            tx.put(obj, "kind", "Ellipse")?;
            tx.put(obj, "center_x", center.x as i64)?;
            tx.put(obj, "center_y", center.y as i64)?;
            tx.put(obj, "radius_x", *radius_x as i64)?;
            tx.put(obj, "radius_y", *radius_y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Freehand { points, char, label, color } => {
            tx.put(obj, "kind", "Freehand")?;
            tx.put(obj, "char", char.to_string())?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
            let points_obj = tx.put_object(obj, "points", ObjType::List)?;
            for (i, point) in points.iter().enumerate() {
                let point_obj = tx.insert_object(&points_obj, i, ObjType::Map)?;
                tx.put(&point_obj, "x", point.x as i64)?;
                tx.put(&point_obj, "y", point.y as i64)?;
            }
        }
        ShapeKind::Text { pos, content, color } => {
            tx.put(obj, "kind", "Text")?;
            tx.put(obj, "pos_x", pos.x as i64)?;
            tx.put(obj, "pos_y", pos.y as i64)?;
            tx.put(obj, "content", content.as_str())?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
        }
        ShapeKind::Triangle { p1, p2, p3, label, color } => {
            tx.put(obj, "kind", "Triangle")?;
            tx.put(obj, "p1_x", p1.x as i64)?;
            tx.put(obj, "p1_y", p1.y as i64)?;
            tx.put(obj, "p2_x", p2.x as i64)?;
            tx.put(obj, "p2_y", p2.y as i64)?;
            tx.put(obj, "p3_x", p3.x as i64)?;
            tx.put(obj, "p3_y", p3.y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Parallelogram { start, end, label, color } => {
            tx.put(obj, "kind", "Parallelogram")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Hexagon { center, radius_x, radius_y, label, color } => {
            tx.put(obj, "kind", "Hexagon")?;
            tx.put(obj, "center_x", center.x as i64)?;
            tx.put(obj, "center_y", center.y as i64)?;
            tx.put(obj, "radius_x", *radius_x as i64)?;
            tx.put(obj, "radius_y", *radius_y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Trapezoid { start, end, label, color } => {
            tx.put(obj, "kind", "Trapezoid")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::RoundedRect { start, end, label, color } => {
            tx.put(obj, "kind", "RoundedRect")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Cylinder { start, end, label, color } => {
            tx.put(obj, "kind", "Cylinder")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Cloud { start, end, label, color } => {
            tx.put(obj, "kind", "Cloud")?;
            tx.put(obj, "start_x", start.x as i64)?;
            tx.put(obj, "start_y", start.y as i64)?;
            tx.put(obj, "end_x", end.x as i64)?;
            tx.put(obj, "end_y", end.y as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
        ShapeKind::Star { center, outer_radius, inner_radius, label, color } => {
            tx.put(obj, "kind", "Star")?;
            tx.put(obj, "center_x", center.x as i64)?;
            tx.put(obj, "center_y", center.y as i64)?;
            tx.put(obj, "outer_radius", *outer_radius as i64)?;
            tx.put(obj, "inner_radius", *inner_radius as i64)?;
            tx.put(obj, "color", shape_color_to_str(*color))?;
            if let Some(l) = label {
                tx.put(obj, "label", l.as_str())?;
            }
        }
    }
    Ok(())
}

fn read_shape_kind(doc: &Automerge, obj: &ObjId) -> Result<Option<ShapeKind>> {
    let kind_value = doc.get(obj, "kind")?;

    let kind_str = match &kind_value {
        Some((automerge::Value::Scalar(s), _)) => {
            // to_string() includes quotes, so trim them
            let s = s.to_string();
            s.trim_matches('"').to_string()
        },
        _ => return Ok(None),
    };

    let kind = match kind_str.as_str() {
        "Line" => ShapeKind::Line {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            style: get_line_style(doc, obj)?,
            start_connection: get_opt_u64(doc, obj, "start_conn")?,
            end_connection: get_opt_u64(doc, obj, "end_conn")?,
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Arrow" => ShapeKind::Arrow {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            style: get_line_style(doc, obj)?,
            start_connection: get_opt_u64(doc, obj, "start_conn")?,
            end_connection: get_opt_u64(doc, obj, "end_conn")?,
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Rectangle" => ShapeKind::Rectangle {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "DoubleBox" => ShapeKind::DoubleBox {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Diamond" => ShapeKind::Diamond {
            center: Position::new(get_i32(doc, obj, "center_x")?, get_i32(doc, obj, "center_y")?),
            half_width: get_i32(doc, obj, "half_width")?,
            half_height: get_i32(doc, obj, "half_height")?,
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Ellipse" => ShapeKind::Ellipse {
            center: Position::new(get_i32(doc, obj, "center_x")?, get_i32(doc, obj, "center_y")?),
            radius_x: get_i32(doc, obj, "radius_x")?,
            radius_y: get_i32(doc, obj, "radius_y")?,
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Freehand" => {
            let char_str = get_string(doc, obj, "char")?;
            let ch = char_str.chars().next().unwrap_or('*');

            let points = match doc.get(obj, "points")? {
                Some((_, points_obj)) => {
                    let len = doc.length(&points_obj);
                    let mut pts = Vec::with_capacity(len);
                    for i in 0..len {
                        if let Some((_, point_obj)) = doc.get(&points_obj, i)? {
                            let x = get_i32(doc, &point_obj, "x")?;
                            let y = get_i32(doc, &point_obj, "y")?;
                            pts.push(Position::new(x, y));
                        }
                    }
                    pts
                }
                None => Vec::new(),
            };

            ShapeKind::Freehand { points, char: ch, label: get_opt_string(doc, obj, "label")?, color: get_shape_color(doc, obj)? }
        }
        "Text" => ShapeKind::Text {
            pos: Position::new(get_i32(doc, obj, "pos_x")?, get_i32(doc, obj, "pos_y")?),
            content: get_string(doc, obj, "content")?,
            color: get_shape_color(doc, obj)?,
        },
        "Triangle" => ShapeKind::Triangle {
            p1: Position::new(get_i32(doc, obj, "p1_x")?, get_i32(doc, obj, "p1_y")?),
            p2: Position::new(get_i32(doc, obj, "p2_x")?, get_i32(doc, obj, "p2_y")?),
            p3: Position::new(get_i32(doc, obj, "p3_x")?, get_i32(doc, obj, "p3_y")?),
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Parallelogram" => ShapeKind::Parallelogram {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Hexagon" => ShapeKind::Hexagon {
            center: Position::new(get_i32(doc, obj, "center_x")?, get_i32(doc, obj, "center_y")?),
            radius_x: get_i32(doc, obj, "radius_x")?,
            radius_y: get_i32(doc, obj, "radius_y")?,
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Trapezoid" => ShapeKind::Trapezoid {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "RoundedRect" => ShapeKind::RoundedRect {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Cylinder" => ShapeKind::Cylinder {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Cloud" => ShapeKind::Cloud {
            start: Position::new(get_i32(doc, obj, "start_x")?, get_i32(doc, obj, "start_y")?),
            end: Position::new(get_i32(doc, obj, "end_x")?, get_i32(doc, obj, "end_y")?),
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        "Star" => ShapeKind::Star {
            center: Position::new(get_i32(doc, obj, "center_x")?, get_i32(doc, obj, "center_y")?),
            outer_radius: get_i32(doc, obj, "outer_radius")?,
            inner_radius: get_i32(doc, obj, "inner_radius")?,
            label: get_opt_string(doc, obj, "label")?,
            color: get_shape_color(doc, obj)?,
        },
        _ => return Ok(None),
    };

    Ok(Some(kind))
}

fn get_i32(doc: &Automerge, obj: &ObjId, key: &str) -> Result<i32> {
    match doc.get(obj, key)? {
        Some((automerge::Value::Scalar(s), _)) => {
            if let Some(n) = s.to_i64() {
                Ok(n as i32)
            } else {
                Err(anyhow!("Expected i64 for key {}", key))
            }
        }
        _ => Err(anyhow!("Missing key {}", key)),
    }
}

fn get_string(doc: &Automerge, obj: &ObjId, key: &str) -> Result<String> {
    match doc.get(obj, key)? {
        Some((automerge::Value::Scalar(s), _)) => Ok(s.to_string().trim_matches('"').to_string()),
        _ => Err(anyhow!("Missing key {}", key)),
    }
}

fn get_opt_string(doc: &Automerge, obj: &ObjId, key: &str) -> Result<Option<String>> {
    match doc.get(obj, key)? {
        Some((automerge::Value::Scalar(s), _)) => Ok(Some(s.to_string().trim_matches('"').to_string())),
        _ => Ok(None),
    }
}

fn get_opt_u64(doc: &Automerge, obj: &ObjId, key: &str) -> Result<Option<u64>> {
    match doc.get(obj, key)? {
        Some((automerge::Value::Scalar(s), _)) => {
            if let Some(n) = s.to_i64() {
                Ok(Some(n as u64))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn get_line_style(doc: &Automerge, obj: &ObjId) -> Result<LineStyle> {
    match doc.get(obj, "style")? {
        Some((automerge::Value::Scalar(s), _)) => {
            let style_str = s.to_string();
            Ok(str_to_line_style(style_str.trim_matches('"')))
        },
        _ => Ok(LineStyle::default()),
    }
}

fn line_style_to_str(style: LineStyle) -> &'static str {
    match style {
        LineStyle::Straight => "Straight",
        LineStyle::OrthogonalHV => "OrthogonalHV",
        LineStyle::OrthogonalVH => "OrthogonalVH",
    }
}

fn str_to_line_style(s: &str) -> LineStyle {
    match s {
        "Straight" => LineStyle::Straight,
        "OrthogonalHV" => LineStyle::OrthogonalHV,
        "OrthogonalVH" => LineStyle::OrthogonalVH,
        _ => LineStyle::default(),
    }
}

fn shape_color_to_str(color: ShapeColor) -> &'static str {
    match color {
        ShapeColor::White => "White",
        ShapeColor::Black => "Black",
        ShapeColor::Red => "Red",
        ShapeColor::Green => "Green",
        ShapeColor::Yellow => "Yellow",
        ShapeColor::Blue => "Blue",
        ShapeColor::Magenta => "Magenta",
        ShapeColor::Cyan => "Cyan",
        ShapeColor::Gray => "Gray",
        ShapeColor::DarkGray => "DarkGray",
        ShapeColor::LightRed => "LightRed",
        ShapeColor::LightGreen => "LightGreen",
        ShapeColor::LightYellow => "LightYellow",
        ShapeColor::LightBlue => "LightBlue",
        ShapeColor::LightMagenta => "LightMagenta",
        ShapeColor::LightCyan => "LightCyan",
    }
}

fn str_to_shape_color(s: &str) -> ShapeColor {
    match s {
        "White" => ShapeColor::White,
        "Black" => ShapeColor::Black,
        "Red" => ShapeColor::Red,
        "Green" => ShapeColor::Green,
        "Yellow" => ShapeColor::Yellow,
        "Blue" => ShapeColor::Blue,
        "Magenta" => ShapeColor::Magenta,
        "Cyan" => ShapeColor::Cyan,
        "Gray" => ShapeColor::Gray,
        "DarkGray" => ShapeColor::DarkGray,
        "LightRed" => ShapeColor::LightRed,
        "LightGreen" => ShapeColor::LightGreen,
        "LightYellow" => ShapeColor::LightYellow,
        "LightBlue" => ShapeColor::LightBlue,
        "LightMagenta" => ShapeColor::LightMagenta,
        "LightCyan" => ShapeColor::LightCyan,
        _ => ShapeColor::default(),
    }
}

fn get_shape_color(doc: &Automerge, obj: &ObjId) -> Result<ShapeColor> {
    match doc.get(obj, "color")? {
        Some((automerge::Value::Scalar(s), _)) => {
            let color_str = s.to_string();
            Ok(str_to_shape_color(color_str.trim_matches('"')))
        },
        _ => Ok(ShapeColor::default()),
    }
}

/// Find which old snap point a position matches and return the corresponding new snap point
fn find_corresponding_snap(pos: &Position, old_snaps: &[Position], new_snaps: &[Position]) -> Option<Position> {
    // Find the closest old snap point to this position
    let mut best_idx = None;
    let mut best_dist = i32::MAX;

    for (idx, old_snap) in old_snaps.iter().enumerate() {
        let dist = (pos.x - old_snap.x).abs() + (pos.y - old_snap.y).abs();
        if dist < best_dist {
            best_dist = dist;
            best_idx = Some(idx);
        }
    }

    // If we found a matching snap point and the new snaps have the same index, return it
    best_idx.and_then(|idx| new_snaps.get(idx).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rect(x: i32, y: i32, w: i32, h: i32) -> ShapeKind {
        ShapeKind::Rectangle {
            start: Position::new(x, y),
            end: Position::new(x + w, y + h),
            color: ShapeColor::default(),
            label: None,
        }
    }

    fn make_line(x1: i32, y1: i32, x2: i32, y2: i32) -> ShapeKind {
        ShapeKind::Line {
            start: Position::new(x1, y1),
            end: Position::new(x2, y2),
            color: ShapeColor::default(),
            style: LineStyle::Straight,
            start_connection: None,
            end_connection: None,
            label: None,
        }
    }

    // --- DocumentId tests ---

    #[test]
    fn document_id_new_unique() {
        let id1 = DocumentId::new();
        let id2 = DocumentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn document_id_display() {
        let id = DocumentId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
        assert!(uuid::Uuid::parse_str(&s).is_ok());
    }

    // --- ShapeId tests ---

    #[test]
    fn shape_id_new_unique() {
        let id1 = ShapeId::new();
        let id2 = ShapeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn shape_id_display() {
        let id = ShapeId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
        assert!(uuid::Uuid::parse_str(&s).is_ok());
    }

    // --- GroupId tests ---

    #[test]
    fn group_id_new_unique() {
        let id1 = GroupId::new();
        let id2 = GroupId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn group_id_display() {
        let id = GroupId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
        assert!(uuid::Uuid::parse_str(&s).is_ok());
    }

    // --- Document tests ---

    #[test]
    fn document_new() {
        let doc = Document::new();
        assert!(!doc.is_dirty());
        assert!(doc.storage_path().is_none());
    }

    #[test]
    fn document_add_shape() {
        let mut doc = Document::new();
        let id = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        assert!(doc.is_dirty());

        let shapes = doc.read_all_shapes().unwrap();
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].0, id);
    }

    #[test]
    fn document_add_multiple_shapes() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();
        let id3 = doc.add_shape(make_line(0, 0, 10, 10)).unwrap();

        let shapes = doc.read_all_shapes().unwrap();
        assert_eq!(shapes.len(), 3);

        let ids: Vec<_> = shapes.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn document_update_shape() {
        let mut doc = Document::new();
        let id = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();

        // Update the shape
        let new_rect = make_rect(5, 5, 15, 10);
        doc.update_shape(id, new_rect.clone()).unwrap();

        let shapes = doc.read_all_shapes().unwrap();
        assert_eq!(shapes.len(), 1);

        // Verify the shape was updated
        if let ShapeKind::Rectangle { start, end, .. } = &shapes[0].1 {
            assert_eq!(start.x, 5);
            assert_eq!(start.y, 5);
            assert_eq!(end.x, 20);
            assert_eq!(end.y, 15);
        } else {
            panic!("Expected Rectangle");
        }
    }

    #[test]
    fn document_delete_shape() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();

        doc.delete_shape(id1).unwrap();

        let shapes = doc.read_all_shapes().unwrap();
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].0, id2);
    }

    // --- Shape order tests ---

    #[test]
    fn document_shape_order() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();
        let id3 = doc.add_shape(make_rect(40, 40, 10, 5)).unwrap();

        let order = doc.read_shape_order().unwrap();
        assert_eq!(order.len(), 3);
        // New shapes are added to the end (top)
        assert_eq!(order[0], id1);
        assert_eq!(order[1], id2);
        assert_eq!(order[2], id3);
    }

    #[test]
    fn document_bring_to_front() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();
        let id3 = doc.add_shape(make_rect(40, 40, 10, 5)).unwrap();

        doc.bring_to_front(&[id1]).unwrap();

        let order = doc.read_shape_order().unwrap();
        assert_eq!(order, vec![id2, id3, id1]);
    }

    #[test]
    fn document_send_to_back() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();
        let id3 = doc.add_shape(make_rect(40, 40, 10, 5)).unwrap();

        doc.send_to_back(&[id3]).unwrap();

        let order = doc.read_shape_order().unwrap();
        assert_eq!(order, vec![id3, id1, id2]);
    }

    #[test]
    fn document_bring_forward() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();
        let id3 = doc.add_shape(make_rect(40, 40, 10, 5)).unwrap();

        doc.bring_forward(&[id1]).unwrap();

        let order = doc.read_shape_order().unwrap();
        assert_eq!(order, vec![id2, id1, id3]);
    }

    #[test]
    fn document_send_backward() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();
        let id3 = doc.add_shape(make_rect(40, 40, 10, 5)).unwrap();

        doc.send_backward(&[id3]).unwrap();

        let order = doc.read_shape_order().unwrap();
        assert_eq!(order, vec![id1, id3, id2]);
    }

    // --- Layer tests ---

    #[test]
    fn document_default_layer() {
        let doc = Document::new();
        let layers = doc.read_all_layers().unwrap();
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].name, "Layer 1");
        assert!(layers[0].visible);
        assert!(!layers[0].locked);
    }

    #[test]
    fn document_create_layer() {
        let mut doc = Document::new();
        let id = doc.create_layer("Layer 2").unwrap();

        let layers = doc.read_all_layers().unwrap();
        assert_eq!(layers.len(), 2);

        let new_layer = layers.iter().find(|l| l.id == id).unwrap();
        assert_eq!(new_layer.name, "Layer 2");
    }

    #[test]
    fn document_delete_layer() {
        let mut doc = Document::new();
        let id1 = doc.create_layer("Layer 2").unwrap();
        let _id2 = doc.create_layer("Layer 3").unwrap();

        doc.delete_layer(id1).unwrap();

        let layers = doc.read_all_layers().unwrap();
        assert_eq!(layers.len(), 2); // Original layer + Layer 3

        assert!(!layers.iter().any(|l| l.id == id1));
    }

    #[test]
    fn document_rename_layer() {
        let mut doc = Document::new();
        let layers = doc.read_all_layers().unwrap();
        let layer_id = layers[0].id;

        doc.rename_layer(layer_id, "Renamed Layer").unwrap();

        let layer = doc.read_layer(layer_id).unwrap().unwrap();
        assert_eq!(layer.name, "Renamed Layer");
    }

    #[test]
    fn document_layer_visibility() {
        let mut doc = Document::new();
        let layers = doc.read_all_layers().unwrap();
        let layer_id = layers[0].id;

        doc.set_layer_visible(layer_id, false).unwrap();

        let layer = doc.read_layer(layer_id).unwrap().unwrap();
        assert!(!layer.visible);
    }

    #[test]
    fn document_layer_locked() {
        let mut doc = Document::new();
        let layers = doc.read_all_layers().unwrap();
        let layer_id = layers[0].id;

        doc.set_layer_locked(layer_id, true).unwrap();

        let layer = doc.read_layer(layer_id).unwrap().unwrap();
        assert!(layer.locked);
    }

    // --- Group tests ---

    #[test]
    fn document_create_group() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();

        let group_id = doc.create_group(&[id1, id2], None).unwrap();

        let group = doc.read_group(group_id).unwrap().unwrap();
        assert_eq!(group.members.len(), 2);
        assert!(group.members.contains(&id1));
        assert!(group.members.contains(&id2));
    }

    #[test]
    fn document_delete_group() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();

        let group_id = doc.create_group(&[id1, id2], None).unwrap();
        doc.delete_group(group_id).unwrap();

        let group = doc.read_group(group_id).unwrap();
        assert!(group.is_none());
    }

    #[test]
    fn document_read_all_groups() {
        let mut doc = Document::new();
        let id1 = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        let id2 = doc.add_shape(make_rect(20, 20, 10, 5)).unwrap();
        let id3 = doc.add_shape(make_rect(40, 40, 10, 5)).unwrap();
        let id4 = doc.add_shape(make_rect(60, 60, 10, 5)).unwrap();

        let _group1 = doc.create_group(&[id1, id2], None).unwrap();
        let _group2 = doc.create_group(&[id3, id4], None).unwrap();

        let groups = doc.read_all_groups().unwrap();
        assert_eq!(groups.len(), 2);
    }

    // --- Persistence tests ---

    #[test]
    fn document_save_and_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_doc.automerge");

        // Create and save document
        let mut doc = Document::new();
        let id = doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        doc.save_to(&file_path).unwrap();

        // Load and verify
        let loaded_doc = Document::load(&file_path).unwrap();
        let shapes = loaded_doc.read_all_shapes().unwrap();
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].0, id);
    }

    #[test]
    fn document_merge() {
        // Create a document and save it, then load two copies to simulate P2P
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("shared.automerge");

        // Create original document with one shape and save
        let mut original = Document::new();
        original.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        original.save_to(&file_path).unwrap();

        // Load two copies (simulating two peers)
        let mut doc1 = Document::load(&file_path).unwrap();
        let mut doc2 = Document::load(&file_path).unwrap();

        // Each peer makes an edit
        doc1.add_shape(make_rect(50, 50, 10, 5)).unwrap();
        doc2.add_shape(make_rect(100, 100, 10, 5)).unwrap();

        // Get doc2's automerge for merge
        let mut doc2_am = doc2.clone_automerge();

        // Merge doc2's changes into doc1
        doc1.merge(&mut doc2_am).unwrap();

        // After merge, doc1 should have all 3 shapes
        let shapes = doc1.read_all_shapes().unwrap();
        assert_eq!(shapes.len(), 3);
    }

    // --- Dirty flag tests ---

    #[test]
    fn document_dirty_flag() {
        let mut doc = Document::new();
        assert!(!doc.is_dirty());

        doc.add_shape(make_rect(0, 0, 10, 5)).unwrap();
        assert!(doc.is_dirty());

        // Save clears dirty flag
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.automerge");
        doc.save_to(&file_path).unwrap();
        assert!(!doc.is_dirty());
    }

    #[test]
    fn document_mark_dirty() {
        let mut doc = Document::new();
        assert!(!doc.is_dirty());

        doc.mark_dirty();
        assert!(doc.is_dirty());
    }

    // --- Helper function tests ---

    #[test]
    fn str_to_shape_color_all_colors() {
        assert_eq!(str_to_shape_color("White"), ShapeColor::White);
        assert_eq!(str_to_shape_color("Black"), ShapeColor::Black);
        assert_eq!(str_to_shape_color("Red"), ShapeColor::Red);
        assert_eq!(str_to_shape_color("Green"), ShapeColor::Green);
        assert_eq!(str_to_shape_color("Yellow"), ShapeColor::Yellow);
        assert_eq!(str_to_shape_color("Blue"), ShapeColor::Blue);
        assert_eq!(str_to_shape_color("Magenta"), ShapeColor::Magenta);
        assert_eq!(str_to_shape_color("Cyan"), ShapeColor::Cyan);
        assert_eq!(str_to_shape_color("Gray"), ShapeColor::Gray);
        assert_eq!(str_to_shape_color("DarkGray"), ShapeColor::DarkGray);
        assert_eq!(str_to_shape_color("LightRed"), ShapeColor::LightRed);
        assert_eq!(str_to_shape_color("LightGreen"), ShapeColor::LightGreen);
        assert_eq!(str_to_shape_color("LightYellow"), ShapeColor::LightYellow);
        assert_eq!(str_to_shape_color("LightBlue"), ShapeColor::LightBlue);
        assert_eq!(str_to_shape_color("LightMagenta"), ShapeColor::LightMagenta);
        assert_eq!(str_to_shape_color("LightCyan"), ShapeColor::LightCyan);
    }

    #[test]
    fn str_to_shape_color_unknown() {
        assert_eq!(str_to_shape_color("Unknown"), ShapeColor::default());
        assert_eq!(str_to_shape_color(""), ShapeColor::default());
    }

    #[test]
    fn find_corresponding_snap_basic() {
        let old_snaps = vec![Position::new(0, 0), Position::new(10, 0), Position::new(5, 5)];
        let new_snaps = vec![Position::new(2, 2), Position::new(12, 2), Position::new(7, 7)];

        // Position closest to old_snaps[0] should map to new_snaps[0]
        let result = find_corresponding_snap(&Position::new(1, 1), &old_snaps, &new_snaps);
        assert_eq!(result, Some(Position::new(2, 2)));

        // Position closest to old_snaps[1] should map to new_snaps[1]
        let result = find_corresponding_snap(&Position::new(9, 0), &old_snaps, &new_snaps);
        assert_eq!(result, Some(Position::new(12, 2)));
    }

    #[test]
    fn find_corresponding_snap_empty() {
        let result = find_corresponding_snap(&Position::new(0, 0), &[], &[]);
        assert_eq!(result, None);
    }
}
