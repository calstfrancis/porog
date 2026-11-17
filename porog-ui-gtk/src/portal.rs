//! Turning a file the sandbox was handed into the path the other app can open.
//!
//! A file dropped on the sandboxed Porog arrives as `/run/user/UID/doc/ID/name`, a document-
//! portal path that exists only for apps the portal has shared it with, which Pereplyot and
//! Zerkalo are not. The portal knows the real path on the host (`GetHostPaths`), and those apps
//! have home access, so that is what they are given.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gtk::{gio, glib};
use gtk4 as gtk;

use glib::prelude::*;

/// The document id in `/run/user/UID/doc/ID/…`, if `path` is one.
fn document_id(path: &Path) -> Option<String> {
    let mut parts = path.components().map(|c| c.as_os_str().to_str());
    let (root, run, user, _uid, doc, id) = (
        parts.next()??,
        parts.next()??,
        parts.next()??,
        parts.next()??,
        parts.next()??,
        parts.next()??,
    );
    (root == "/" && run == "run" && user == "user" && doc == "doc").then(|| id.to_string())
}

/// The host path for a file `path` handed to this process. A path that is not a portal path
/// is already a real one and comes back unchanged. `None` when the portal cannot say.
/// Blocks on D-Bus, so call it from a worker thread.
pub fn host_path(path: &Path) -> Option<PathBuf> {
    let Some(id) = document_id(path) else {
        return Some(path.to_path_buf());
    };
    let mut id_bytes = id.clone().into_bytes();
    id_bytes.push(0);
    let connection = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()?;
    let reply = connection
        .call_sync(
            Some("org.freedesktop.portal.Documents"),
            "/org/freedesktop/portal/documents",
            "org.freedesktop.portal.Documents",
            "GetHostPaths",
            Some(&(vec![id_bytes],).to_variant()),
            Some(glib::VariantTy::new("(a{say})").ok()?),
            gio::DBusCallFlags::NONE,
            2000,
            gio::Cancellable::NONE,
        )
        .ok()?;
    let (paths,): (HashMap<String, Vec<u8>>,) = reply.get()?;
    let mut bytes = paths.get(&id)?.clone();
    if bytes.last() == Some(&0) {
        bytes.pop();
    }
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_portal_paths_have_a_document_id() {
        assert_eq!(
            document_id(Path::new("/run/user/1000/doc/a1b2c3d4/essay.typ")).as_deref(),
            Some("a1b2c3d4")
        );
        assert_eq!(document_id(Path::new("/home/cal/essay.typ")), None);
        assert_eq!(document_id(Path::new("/run/user/1000/doc")), None);
    }
}
