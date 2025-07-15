use std::{
    ffi::CStr,
    io::{self, BufRead, Read, Write},
};

use anyhow::{Context, bail};

use crate::objects::{Kind, Object};

pub(crate) fn invoke(name_only: bool, tree_hash: &str) -> anyhow::Result<()> {
    let mut object = Object::read(tree_hash).context("parse out tree object file")?;
    match object.kind {
        Kind::Tree => {
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            let mut buf = Vec::new();
            let mut hash = [0; 20];
            loop {
                buf.clear();
                let n = object
                    .reader
                    .read_until(0, &mut buf)
                    .context("read tree object entry")?;
                if n == 0 {
                    break;
                }
                object
                    .reader
                    .read_exact(&mut hash)
                    .context("read tree object entry hash")?;

                let mode_name = unsafe { CStr::from_bytes_with_nul_unchecked(&buf) };
                let mut bits = mode_name.to_bytes().splitn(2, |&c| c == b' ');
                let mode = bits.next().expect("splice always yields once");
                let name = bits
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("tree entry does not contain a file name"))?;

                if name_only {
                    stdout.write_all(name)?;
                } else {
                    let mode = unsafe { str::from_utf8_unchecked(mode) };
                    let hash = hex::encode(&hash);
                    let kind = Object::read(&hash)
                        .with_context(|| format!("read object for hash `{}`", hash))?
                        .kind;
                    write!(stdout, "{mode:0>6} {kind} {hash}\t")
                        .context("write tree entry meta to stdout")?;
                    stdout.write_all(name)?;
                }
                writeln!(stdout, "").context("write newline to stdout")?;
            }
        }
        _ => bail!("We don't supported the {}", object.kind),
    };

    Ok(())
}
