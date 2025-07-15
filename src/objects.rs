use std::ffi::CStr;
use std::fmt::Display;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

use anyhow::Context;
use anyhow::bail;
use flate2::read::ZlibDecoder;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Blob,
    Tree,
    Commit,
}

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Blob => write!(f, "blob"),
            Kind::Tree => write!(f, "tree"),
            Kind::Commit => write!(f, "commit"),
        }
    }
}

pub(crate) struct Object<R> {
    pub(crate) kind: Kind,
    pub(crate) expected_size: u64,
    pub(crate) reader: R,
}

impl Object<()> {
    pub(crate) fn read(object_hash: &str) -> anyhow::Result<Object<impl BufRead>> {
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
            "tree" => Kind::Tree,
            "commit" => Kind::Commit,
            _ => bail!(".git objects did not start with a known type: `{kind}`"),
        };

        let size = size
            .parse()
            .with_context(|| format!(".git/objects file header has invalid size: {size}"))?;
        let z = z.take(size);
        Ok(Object {
            kind,
            expected_size: size,
            reader: z,
        })
    }
}
