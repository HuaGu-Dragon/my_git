use std::path::PathBuf;

use anyhow::Context;
use std::io;

use crate::objects::Object;

pub(crate) fn invoke(write: bool, file: &PathBuf) -> anyhow::Result<()> {
    let object = Object::blob_from_file(file).context("open blob from file")?;
    let hash = if write {
        object.write_to_objects().context("write blob object")?
    } else {
        object.write(io::sink()).context("write blob object")?
    };
    let hash = hex::encode(hash);

    println!("{hash}");
    Ok(())
}
