use std::{fs, io::Cursor, path::Path};

use anyhow::{Context, Ok};

use crate::objects::{Kind, Object};

#[allow(unused)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    #[cfg(not(target_os = "windows"))]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(target_os = "windows")]
    {
        false
    }
}

pub(crate) fn write_tree_for(path: &Path) -> anyhow::Result<Option<[u8; 20]>> {
    let mut dir =
        fs::read_dir(path).with_context(|| format!("read directory `{}`", path.display()))?;
    let mut entries = Vec::new();
    while let Some(entry) = dir.next() {
        let entry = entry.with_context(|| format!("bad directory entry in {}", path.display()))?;
        let name = entry.file_name();
        let meta = entry.metadata().context("metadata for directory entry")?;
        entries.push((entry, name, meta))
    }
    entries.sort_unstable_by(|a, b| {
        let afn = &a.1;
        let afn = afn.as_encoded_bytes();
        let bfn = &b.1;
        let bfn = bfn.as_encoded_bytes();
        let len = std::cmp::min(afn.len(), bfn.len());
        match afn[..len].cmp(&bfn[..len]) {
            std::cmp::Ordering::Equal => {}
            o => return o,
        };
        if afn.len() == bfn.len() {
            return std::cmp::Ordering::Equal;
        }
        let c1 = if let Some(c) = afn.get(len).copied() {
            Some(c)
        } else if a.2.is_dir() {
            Some(b'/')
        } else {
            None
        };
        let c2 = if let Some(c) = bfn.get(len).copied() {
            Some(c)
        } else if b.2.is_dir() {
            Some(b'/')
        } else {
            None
        };
        c1.cmp(&c2)
    });

    let mut tree_object = Vec::new();
    for (entry, name, metadata) in entries {
        if name == ".git" {
            continue; // Skip the .git directory
        }
        let path = entry.path();
        let mode = if metadata.is_dir() {
            "40000" // directory
        } else if metadata.is_symlink() {
            "120000" // symlink
        } else if is_executable(&metadata) {
            "100755" // executable file
        } else {
            "100644" // regular file
        };
        let hash = if metadata.is_dir() {
            let Some(hash) = write_tree_for(&path)? else {
                continue;
            };
            hash
        } else {
            Object::blob_from_file(&path)
                .context("construct blob object from file")?
                .write_to_objects()
                .context("write")?
        };
        tree_object.extend(mode.as_bytes());
        tree_object.extend(b" ");
        tree_object.extend(name.as_encoded_bytes());
        tree_object.push(0); // null terminator
        tree_object.extend(hash);
    }
    if tree_object.is_empty() {
        return Ok(None);
    } else {
        Ok(Some(
            Object {
                kind: Kind::Tree,
                expected_size: tree_object.len() as u64,
                reader: Cursor::new(tree_object),
            }
            .write_to_objects()
            .context("write tree object")?,
        ))
    }
}

pub(crate) fn invoke() -> anyhow::Result<()> {
    let Some(hash) = write_tree_for(Path::new("./")).context("write tree for current directory")?
    else {
        anyhow::bail!("empty tree object");
    };
    println!("{}", hex::encode(hash));
    Ok(())
}
