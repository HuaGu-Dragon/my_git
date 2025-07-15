use std::ffi::CStr;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

use anyhow::Context;
use anyhow::bail;
use flate2::read::ZlibDecoder;
use std::io;

enum Kind {
    Blob,
}

pub(crate) fn invoke(pretty_print: bool, object_hash: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        pretty_print,
        "that what git did, you need to give mode or -p, but I only support -p now. qwq"
    );

    //TODO: Implement short object_hash to full object hash conversion
    let f = std::fs::File::open(format!(
        ".git/objects/{}/{}",
        &object_hash[..2],
        &object_hash[2..]
    ))
    .context("open object file from .git/objects")?;
    let z = ZlibDecoder::new(f);
    let mut z = BufReader::new(z);
    let mut buf = Vec::new();
    z.read_until(0, &mut buf)
        .context("read header from .git/objects")?;

    let header = unsafe { CStr::from_bytes_with_nul_unchecked(&buf) };

    let header = header
        .to_str()
        .context(".git/objects file header isn't valid UTF-8")?;

    let Some((kind, size)) = header.split_once(' ') else {
        bail!(".git objects did not start with a known type: `{header}`");
    };

    let kind = match kind {
        "blob" => Kind::Blob,
        _ => bail!("We don't implement the {kind}"),
    };

    let size = size
        .parse()
        .with_context(|| format!(".git/objects file header has invalid size: {size}"))?;
    let mut z = z.take(size);
    match kind {
        Kind::Blob => {
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            let n = io::copy(&mut z, &mut stdout)
                .context("copy .git/objects file content to stdout")?;
            anyhow::ensure!(
                n == size,
                "expected to read {size} bytes, but read {n} bytes"
            );
        }
    };
    Ok(())
}
