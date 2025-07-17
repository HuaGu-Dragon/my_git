use anyhow::Context;
use anyhow::bail;
use std::io;

use crate::objects::Kind;
use crate::objects::Object;

pub(crate) fn invoke(pretty_print: bool, object_hash: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        pretty_print,
        "that what git did, you need to give mode or -p, but I only support -p now. qwq"
    );

    let mut object = Object::read(object_hash).context("parse out blob object file")?;
    match object.kind {
        Kind::Blob => {
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            let n = io::copy(&mut object.reader, &mut stdout)
                .context("copy .git/objects file content to stdout")?;
            anyhow::ensure!(
                n == object.expected_size,
                "expected to read {} bytes, but read {n} bytes",
                object.expected_size
            );
        }
        _ => bail!("We don't supported the {}", object.kind),
    };
    Ok(())
}
