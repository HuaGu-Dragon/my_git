use std::{fmt::Write, io::Cursor, time::SystemTime};

use anyhow::Context;

use crate::objects::{Kind, Object};

pub(crate) fn invoke(
    message: String,
    tree_hash: &str,
    parent_hash: Option<&str>,
) -> anyhow::Result<()> {
    let mut commit = String::new();
    writeln!(commit, "tree {tree_hash}")?;
    if let Some(parent) = parent_hash {
        writeln!(commit, "parent {parent}")?;
    }
    let (name, email) =
        if let (Some(name), Some(email)) = (std::env::var_os("NAME"), std::env::var_os("EMAIL")) {
            let name = name
                .into_string()
                .map_err(|_| anyhow::anyhow!("$NAME is invalid"))?;
            let email = email
                .into_string()
                .map_err(|_| anyhow::anyhow!("$EMAIL is invalid"))?;
            (name, email)
        } else {
            (String::from("my_git"), String::from("myemail@example.com"))
        };
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .context("get current time")?;
    writeln!(commit, "author {name} <{email}> {} +0800", time.as_secs())?;
    writeln!(
        commit,
        "committer {name} <{email}> {} +0800",
        time.as_secs()
    )?;
    writeln!(commit, "")?;
    writeln!(commit, "{message}")?;

    let hash = Object {
        kind: Kind::Commit,
        expected_size: commit.len() as u64,
        reader: Cursor::new(commit),
    }
    .write_to_objects()
    .context("write commit object")?;
    println!("{}", hex::encode(hash));
    Ok(())
}
