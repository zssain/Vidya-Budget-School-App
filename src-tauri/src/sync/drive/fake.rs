//! A fake Google Drive for tests (prompts/P06 Step 9): an in-memory tree that
//! implements [`DriveApi`] and enforces the §11 permission model, with injectable
//! failures for the Step 8 matrix. It stands in for the real `drive.file` client
//! so the exchange logic is provable without Google (and without the Step-0 spike).
//!
//! Permissions follow Drive's cascading share model: a grant on a folder applies
//! to its whole subtree, and a more specific grant wins (max access). The §11
//! layout — `backups/` (Principal only), `exchange/` (reader: all staff),
//! `exchange/ops-<device>/` (writer: that device), `exchange/acks/` (server
//! writes) — is built by [`FakeDrive::provision_school`]. Because the same access
//! check governs every mutation, a staff actor can never write or delete outside
//! its own `ops-` folder, and never touch `backups/` — via the "API" or (modelled
//! identically) the Drive web UI.

use super::{DriveApi, DriveError, DriveFile, DriveResult};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

/// Actor id for the Principal / school server (owner of the whole tree).
pub const PRINCIPAL: &str = "principal";

/// Actor id for the v2 **one shared account** (prompts/P12 §11): every staff device
/// signs into the SAME Google sync account, so they all act with this one identity
/// and share full access — there is no per-staff folder sharing.
pub const SYNC_ACCOUNT: &str = "sync-account";

/// Drive access levels (least → most).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Access {
    Reader,
    Writer,
    Owner,
}

struct Node {
    id: String,
    name: String,
    parent: Option<String>,
    is_folder: bool,
    bytes: Vec<u8>,
    properties: BTreeMap<String, String>,
}

struct Grant {
    actor: String,
    folder_id: String,
    access: Access,
}

#[derive(Default)]
struct Faults {
    quota_full: bool,
    rate_limited: bool,
    revoked_actors: HashSet<String>,
    deleted: HashSet<String>,
}

struct Inner {
    nodes: BTreeMap<String, Node>,
    grants: Vec<Grant>,
    faults: Faults,
    next: u64,
    root: String,
}

/// The shared fake Drive. Clone to share the same backend (it is `Arc`-backed).
#[derive(Clone)]
pub struct FakeDrive {
    inner: Arc<Mutex<Inner>>,
}

/// The §11 folder ids after provisioning.
#[derive(Debug, Clone)]
pub struct SchoolLayout {
    pub root: String,
    pub backups: String,
    pub exchange: String,
    pub acks: String,
    /// device_id → its `exchange/ops-<device>/` folder id.
    pub ops: BTreeMap<String, String>,
}

impl FakeDrive {
    pub fn new() -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "root".to_string(),
            Node {
                id: "root".to_string(),
                name: "Vidya".to_string(),
                parent: None,
                is_folder: true,
                bytes: vec![],
                properties: BTreeMap::new(),
            },
        );
        let inner = Inner {
            nodes,
            grants: vec![Grant { actor: PRINCIPAL.to_string(), folder_id: "root".to_string(), access: Access::Owner }],
            faults: Faults::default(),
            next: 1,
            root: "root".to_string(),
        };
        FakeDrive { inner: Arc::new(Mutex::new(inner)) }
    }

    pub fn root_id(&self) -> String {
        self.inner.lock().unwrap().root.clone()
    }

    /// A client acting as `actor` (each staff device drives Drive with its own token).
    pub fn as_actor(&self, actor: &str) -> FakeDriveClient {
        FakeDriveClient { drive: self.clone(), actor: actor.to_string() }
    }

    /// Build the §11 layout and share it: `backups/` stays Principal-only,
    /// `exchange/` is reader for every staff actor, each device's `ops-<device>/`
    /// is writer for its owner, and `acks/` is written by the server (reader for
    /// staff). `staff_devices` = `(actor, device_id)`.
    pub fn provision_school(&self, staff_devices: &[(&str, &str)]) -> SchoolLayout {
        let mut g = self.inner.lock().unwrap();
        let root = g.root.clone();
        let backups = g.mk_folder(&root, "backups");
        let exchange = g.mk_folder(&root, "exchange");
        let acks = g.mk_folder(&exchange, "acks");
        // every staff actor reads all of exchange/
        let actors: HashSet<&str> = staff_devices.iter().map(|(a, _)| *a).collect();
        for a in &actors {
            g.grants.push(Grant { actor: a.to_string(), folder_id: exchange.clone(), access: Access::Reader });
        }
        let mut ops = BTreeMap::new();
        for (actor, device) in staff_devices {
            let folder = g.mk_folder(&exchange, &format!("ops-{device}"));
            g.grants.push(Grant { actor: actor.to_string(), folder_id: folder.clone(), access: Access::Writer });
            ops.insert((*device).to_string(), folder);
        }
        SchoolLayout { root, backups, exchange, acks, ops }
    }

    /// Provision the v2 **one shared account** layout (prompts/P12 §11, Step 1):
    /// a single Google account owns the whole `Vidya/<school>/` tree and every
    /// device signs into that SAME account (act as [`SYNC_ACCOUNT`]), so there is
    /// **no per-staff folder sharing** — one identity with full access. Each device
    /// still gets its own `exchange/ops-<device>/` folder for its bundles.
    ///
    /// (In production `backups/` lives in a SEPARATE private backup account; the
    /// harness keeps a `backups` folder in the one tree only so the backup-upload
    /// tests have a target.)
    pub fn provision_shared_account(&self, devices: &[&str]) -> SchoolLayout {
        let mut g = self.inner.lock().unwrap();
        let root = g.root.clone();
        // The sync account owns the entire tree (one identity, full access).
        g.grants.push(Grant { actor: SYNC_ACCOUNT.to_string(), folder_id: root.clone(), access: Access::Owner });
        let exchange = g.mk_folder(&root, "exchange");
        let acks = g.mk_folder(&exchange, "acks");
        let backups = g.mk_folder(&root, "backups");
        let mut ops = BTreeMap::new();
        for device in devices {
            ops.insert((*device).to_string(), g.mk_folder(&exchange, &format!("ops-{device}")));
        }
        SchoolLayout { root, backups, exchange, acks, ops }
    }

    /// Share `folder_id` with `actor` at `access` (Step 2 sharing on invite).
    pub fn share(&self, folder_id: &str, actor: &str, access: Access) {
        let mut g = self.inner.lock().unwrap();
        g.grants.push(Grant { actor: actor.to_string(), folder_id: folder_id.to_string(), access });
    }

    /// Remove every grant `actor` has on exactly `folder_id` (Step 2 unshare on
    /// remove/suspend). Access via a broader ancestor grant still applies.
    pub fn unshare(&self, folder_id: &str, actor: &str) {
        let mut g = self.inner.lock().unwrap();
        g.grants.retain(|gr| !(gr.folder_id == folder_id && gr.actor == actor));
    }

    // ---- fault injection (Step 8) ------------------------------------------
    pub fn set_quota_full(&self, on: bool) {
        self.inner.lock().unwrap().faults.quota_full = on;
    }
    pub fn set_rate_limited(&self, on: bool) {
        self.inner.lock().unwrap().faults.rate_limited = on;
    }
    pub fn revoke_token(&self, actor: &str) {
        self.inner.lock().unwrap().faults.revoked_actors.insert(actor.to_string());
    }
    /// Mark a folder (and thus its contents) as deleted/unshared out from under us.
    pub fn delete_underneath(&self, folder_id: &str) {
        self.inner.lock().unwrap().faults.deleted.insert(folder_id.to_string());
    }
}

impl Default for FakeDrive {
    fn default() -> Self {
        Self::new()
    }
}

impl Inner {
    fn new_id(&mut self, is_folder: bool) -> String {
        let id = format!("{}{}", if is_folder { "fld-" } else { "file-" }, self.next);
        self.next += 1;
        id
    }

    /// Create a folder as the owner (no permission check — provisioning path).
    fn mk_folder(&mut self, parent: &str, name: &str) -> String {
        let id = self.new_id(true);
        self.nodes.insert(
            id.clone(),
            Node {
                id: id.clone(),
                name: name.to_string(),
                parent: Some(parent.to_string()),
                is_folder: true,
                bytes: vec![],
                properties: BTreeMap::new(),
            },
        );
        id
    }

    /// The folder chain to check access against: for a folder, itself then its
    /// ancestors; for a file, its parent then ancestors — up to the root.
    fn chain(&self, node_id: &str) -> Vec<String> {
        let mut out = Vec::new();
        let Some(node) = self.nodes.get(node_id) else { return out };
        let mut cur = if node.is_folder { Some(node.id.clone()) } else { node.parent.clone() };
        while let Some(id) = cur {
            out.push(id.clone());
            cur = self.nodes.get(&id).and_then(|n| n.parent.clone());
        }
        out
    }

    /// Effective (max) access `actor` has on `node_id`, via any ancestor grant.
    fn access(&self, actor: &str, node_id: &str) -> Option<Access> {
        let chain: HashSet<String> = self.chain(node_id).into_iter().collect();
        self.grants
            .iter()
            .filter(|g| g.actor == actor && chain.contains(&g.folder_id))
            .map(|g| g.access)
            .max()
    }

    fn is_deleted(&self, node_id: &str) -> bool {
        // The node itself, or any folder in its chain, marked deleted.
        if self.faults.deleted.contains(node_id) {
            return true;
        }
        self.chain(node_id).iter().any(|f| self.faults.deleted.contains(f))
    }

    fn precheck(&self, actor: &str) -> DriveResult<()> {
        if self.faults.revoked_actors.contains(actor) {
            return Err(DriveError::TokenRevoked);
        }
        if self.faults.rate_limited {
            return Err(DriveError::RateLimited);
        }
        Ok(())
    }

    fn to_file(&self, node: &Node) -> DriveFile {
        let checksum = if node.is_folder {
            String::new()
        } else {
            let mut h = Sha256::new();
            h.update(&node.bytes);
            hex(&h.finalize())
        };
        DriveFile {
            id: node.id.clone(),
            name: node.name.clone(),
            parent: node.parent.clone(),
            is_folder: node.is_folder,
            size: node.bytes.len(),
            checksum,
            properties: node.properties.clone(),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A [`DriveApi`] view bound to one acting identity.
pub struct FakeDriveClient {
    drive: FakeDrive,
    actor: String,
}

impl FakeDriveClient {
    fn need(&self, node_id: &str, want: Access, g: &Inner) -> DriveResult<()> {
        if !g.nodes.contains_key(node_id) || g.is_deleted(node_id) {
            return Err(DriveError::NotFound);
        }
        match g.access(&self.actor, node_id) {
            Some(a) if a >= want => Ok(()),
            _ => Err(DriveError::PermissionDenied),
        }
    }
}

impl DriveApi for FakeDriveClient {
    fn list(&self, folder_id: &str) -> DriveResult<Vec<DriveFile>> {
        let g = self.drive.inner.lock().unwrap();
        g.precheck(&self.actor)?;
        self.need(folder_id, Access::Reader, &g)?;
        let mut out: Vec<DriveFile> = g
            .nodes
            .values()
            .filter(|n| n.parent.as_deref() == Some(folder_id))
            .map(|n| g.to_file(n))
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name)); // hlc-prefixed names sort chronologically
        Ok(out)
    }

    fn metadata(&self, file_id: &str) -> DriveResult<DriveFile> {
        let g = self.drive.inner.lock().unwrap();
        g.precheck(&self.actor)?;
        self.need(file_id, Access::Reader, &g)?;
        Ok(g.to_file(g.nodes.get(file_id).unwrap()))
    }

    fn download(&self, file_id: &str) -> DriveResult<Vec<u8>> {
        let g = self.drive.inner.lock().unwrap();
        g.precheck(&self.actor)?;
        self.need(file_id, Access::Reader, &g)?;
        Ok(g.nodes.get(file_id).unwrap().bytes.clone())
    }

    fn create(
        &self,
        parent: &str,
        name: &str,
        bytes: &[u8],
        properties: &BTreeMap<String, String>,
    ) -> DriveResult<DriveFile> {
        let mut g = self.drive.inner.lock().unwrap();
        g.precheck(&self.actor)?;
        self.need(parent, Access::Writer, &g)?;
        if g.faults.quota_full {
            return Err(DriveError::QuotaFull);
        }
        if g
            .nodes
            .values()
            .any(|n| n.parent.as_deref() == Some(parent) && n.name == name)
        {
            return Err(DriveError::NameConflict);
        }
        let id = g.new_id(false);
        let node = Node {
            id: id.clone(),
            name: name.to_string(),
            parent: Some(parent.to_string()),
            is_folder: false,
            bytes: bytes.to_vec(),
            properties: properties.clone(),
        };
        let file = g.to_file(&node);
        g.nodes.insert(id, node);
        Ok(file)
    }

    fn rename(&self, file_id: &str, new_name: &str) -> DriveResult<DriveFile> {
        let mut g = self.drive.inner.lock().unwrap();
        g.precheck(&self.actor)?;
        let parent = g.nodes.get(file_id).and_then(|n| n.parent.clone());
        match parent {
            Some(p) => self.need(&p, Access::Writer, &g)?,
            None => return Err(DriveError::PermissionDenied),
        }
        if g.is_deleted(file_id) || !g.nodes.contains_key(file_id) {
            return Err(DriveError::NotFound);
        }
        g.nodes.get_mut(file_id).unwrap().name = new_name.to_string();
        Ok(g.to_file(g.nodes.get(file_id).unwrap()))
    }

    fn move_to(&self, file_id: &str, new_parent: &str) -> DriveResult<DriveFile> {
        let mut g = self.drive.inner.lock().unwrap();
        g.precheck(&self.actor)?;
        let old_parent = g.nodes.get(file_id).and_then(|n| n.parent.clone());
        match old_parent {
            Some(p) => self.need(&p, Access::Writer, &g)?,
            None => return Err(DriveError::PermissionDenied),
        }
        self.need(new_parent, Access::Writer, &g)?;
        g.nodes.get_mut(file_id).unwrap().parent = Some(new_parent.to_string());
        Ok(g.to_file(g.nodes.get(file_id).unwrap()))
    }

    fn delete(&self, file_id: &str) -> DriveResult<()> {
        let mut g = self.drive.inner.lock().unwrap();
        g.precheck(&self.actor)?;
        let parent = g.nodes.get(file_id).and_then(|n| n.parent.clone());
        match parent {
            Some(p) => self.need(&p, Access::Writer, &g)?,
            None => return Err(DriveError::PermissionDenied),
        }
        g.nodes.remove(file_id);
        Ok(())
    }

    fn ensure_folder(&self, parent: &str, name: &str) -> DriveResult<String> {
        let mut g = self.drive.inner.lock().unwrap();
        g.precheck(&self.actor)?;
        self.need(parent, Access::Writer, &g)?;
        if let Some(existing) = g
            .nodes
            .values()
            .find(|n| n.is_folder && n.parent.as_deref() == Some(parent) && n.name == name)
        {
            return Ok(existing.id.clone());
        }
        Ok(g.mk_folder(parent, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    /// Provision a school with a Principal server + two teachers on two devices.
    fn school() -> (FakeDrive, SchoolLayout) {
        let d = FakeDrive::new();
        let layout = d.provision_school(&[("teacherA", "devA"), ("teacherB", "devB")]);
        (d, layout)
    }

    #[test]
    fn staff_can_read_exchange_and_write_only_their_own_folder() {
        let (d, l) = school();
        let a = d.as_actor("teacherA");
        let opsa = &l.ops["devA"];
        let opsb = &l.ops["devB"];

        // teacherA writes into its own ops folder
        let f = a.create(opsa, "temp.vop", b"hello", &props()).unwrap();
        assert_eq!(a.download(&f.id).unwrap(), b"hello");
        // and can rename (temp → final commit)
        a.rename(&f.id, "final.vop").unwrap();

        // teacherA can LIST and DOWNLOAD teacherB's folder (reader on exchange)
        let b = d.as_actor("teacherB");
        let bf = b.create(opsb, "b.vop", b"world", &props()).unwrap();
        assert!(a.list(opsb).unwrap().iter().any(|x| x.id == bf.id));
        assert_eq!(a.download(&bf.id).unwrap(), b"world");

        // but teacherA CANNOT write or delete in teacherB's folder
        assert_eq!(a.create(opsb, "x.vop", b"x", &props()), Err(DriveError::PermissionDenied));
        assert_eq!(a.delete(&bf.id), Err(DriveError::PermissionDenied));
    }

    #[test]
    fn staff_can_never_touch_backups_via_api_or_ui() {
        // "Staff cannot delete backups/ via API or Drive UI" — one permission
        // check governs both surfaces in the model.
        let (d, l) = school();
        // The Principal (owner) puts a backup there.
        let p = d.as_actor(PRINCIPAL);
        let bk = p.create(&l.backups, "2026-09-24.vidbak", b"BAK", &props()).unwrap();

        let a = d.as_actor("teacherA");
        assert_eq!(a.list(&l.backups), Err(DriveError::PermissionDenied));
        assert_eq!(a.download(&bk.id), Err(DriveError::PermissionDenied));
        assert_eq!(a.delete(&bk.id), Err(DriveError::PermissionDenied));
        assert_eq!(a.create(&l.backups, "evil", b"x", &props()), Err(DriveError::PermissionDenied));
    }

    #[test]
    fn unsharing_removes_access() {
        let (d, l) = school();
        let a = d.as_actor("teacherA");
        assert!(a.list(&l.exchange).is_ok());
        assert!(a.list(&l.ops["devB"]).is_ok()); // reachable via the exchange reader grant
        d.unshare(&l.exchange, "teacherA"); // remove on suspend
        assert_eq!(a.list(&l.exchange), Err(DriveError::PermissionDenied));
        assert_eq!(a.list(&l.ops["devB"]), Err(DriveError::PermissionDenied)); // exchange path gone
        // ...but the device keeps the direct Writer grant on its OWN folder.
        assert!(a.list(&l.ops["devA"]).is_ok());
    }

    #[test]
    fn server_archives_processed_bundles() {
        let (d, l) = school();
        let a = d.as_actor("teacherA");
        let f = a.create(&l.ops["devA"], "b.vop", b"data", &props()).unwrap();
        // Server (owner) makes a _done/ under the device folder and moves it.
        let p = d.as_actor(PRINCIPAL);
        let done = p.ensure_folder(&l.ops["devA"], "_done").unwrap();
        p.move_to(&f.id, &done).unwrap();
        assert_eq!(p.metadata(&f.id).unwrap().parent.as_deref(), Some(done.as_str()));
    }

    #[test]
    fn injected_failures_surface_as_specific_errors() {
        let (d, l) = school();
        let a = d.as_actor("teacherA");
        let ops = &l.ops["devA"];

        d.set_quota_full(true);
        assert_eq!(a.create(ops, "q.vop", b"x", &props()), Err(DriveError::QuotaFull));
        d.set_quota_full(false);

        d.set_rate_limited(true);
        assert_eq!(a.list(ops), Err(DriveError::RateLimited));
        d.set_rate_limited(false);

        d.revoke_token("teacherA");
        assert_eq!(a.list(ops), Err(DriveError::TokenRevoked));

        // A different actor is unaffected by A's revoked token.
        let b = d.as_actor("teacherB");
        assert!(b.list(&l.exchange).is_ok());

        // Folder deleted out from under us → NotFound.
        d.delete_underneath(&l.ops["devB"]);
        assert_eq!(b.list(&l.ops["devB"]), Err(DriveError::NotFound));
    }

    #[test]
    fn duplicate_name_is_a_conflict_so_retry_is_safe() {
        // Retry safety (Step 4): same file name = same content, never a duplicate op.
        let (d, l) = school();
        let a = d.as_actor("teacherA");
        a.create(&l.ops["devA"], "same.vop", b"one", &props()).unwrap();
        assert_eq!(
            a.create(&l.ops["devA"], "same.vop", b"one", &props()),
            Err(DriveError::NameConflict)
        );
    }

    // ----- v2: one shared account (prompts/P12 §11, Step 11) -----

    #[test]
    fn shared_account_devices_read_and_write_each_others_bundles() {
        // Two phones signed into the SAME sync account → one identity, full access,
        // NO per-staff sharing. Each writes into its own ops folder and can read the
        // other's (this is the cross-device behaviour the Step 0 spike must confirm
        // on real Google; here it is proven against the fake shared account).
        let d = FakeDrive::new();
        let l = d.provision_shared_account(&["devA", "devB"]);
        let acc = d.as_actor(SYNC_ACCOUNT);

        acc.create(&l.ops["devA"], "a.vop", b"from-A", &props()).unwrap();
        let fb = acc.create(&l.ops["devB"], "b.vop", b"from-B", &props()).unwrap();
        // Device A (same account) lists + downloads device B's bundle.
        assert!(acc.list(&l.ops["devB"]).unwrap().iter().any(|x| x.id == fb.id));
        assert_eq!(acc.download(&fb.id).unwrap(), b"from-B");
        // The server writes an ack + the epoch marker at the exchange root.
        acc.create(&l.acks, "devA.json", b"{}", &props()).unwrap();
        acc.create(&l.exchange, "epoch.json", b"{}", &props()).unwrap();
    }

    #[test]
    fn epoch_json_through_drive_fences_the_old_server() {
        // Full path: the new server signs epoch.json (epoch 2), writes it to the
        // shared account's exchange/, the old server (epoch 1) reads + verifies it
        // and is fenced to read-only. Combines vidya-core crypto with the fake Drive.
        use vidya_core::epoch::{fence_outcome, sign_epoch, verify_epoch, EpochFile, FenceOutcome, SignedEpoch};

        let seed = [9u8; 32];
        let public = ed25519_dalek::SigningKey::from_bytes(&seed).verifying_key().to_bytes();

        let d = FakeDrive::new();
        let l = d.provision_shared_account(&["srv"]);
        let acc = d.as_actor(SYNC_ACCOUNT);

        let marker = sign_epoch(
            &EpochFile {
                school_id: "s".into(),
                server_epoch: 2,
                server_machine_code: "7KQ2-M9XD-4TRA-P".into(),
                at: "2026-09-25T00:00:00Z".into(),
            },
            &seed,
        );
        acc.create(&l.exchange, "epoch.json", &serde_json::to_vec(&marker).unwrap(), &props()).unwrap();

        // Old server reads it on import.
        let ef = acc.list(&l.exchange).unwrap().into_iter().find(|f| f.name == "epoch.json").unwrap();
        let raw = acc.download(&ef.id).unwrap();
        let signed: SignedEpoch = serde_json::from_slice(&raw).unwrap();
        let verified = verify_epoch(&signed, &public).unwrap();
        assert_eq!(verified.server_epoch, 2);
        // Own epoch is 1 → the file's epoch is higher → this PC is fenced.
        assert_eq!(fence_outcome(1, verified.server_epoch), FenceOutcome::Fenced);
    }
}
