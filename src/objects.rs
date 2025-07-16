use std::ffi::CStr;
use std::fmt::Display;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use anyhow::Context;
use anyhow::bail;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use sha1::{Digest, Sha1};
use std::io;

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
    pub(crate) fn blob_from_file(path: impl AsRef<Path>) -> anyhow::Result<Object<impl Read>> {
        let path = path.as_ref();
        let stat =
            std::fs::metadata(path).with_context(|| format!("stat file `{}`", path.display()))?;

        let file =
            std::fs::File::open(path).with_context(|| format!("open file {}", path.display()))?;

        Ok(Object {
            kind: Kind::Blob,
            expected_size: stat.len(),
            reader: file,
        })
    }
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

impl<R: Read> Object<R> {
    pub(crate) fn write(mut self, writer: impl Write) -> anyhow::Result<[u8; 20]> {
        let e = ZlibEncoder::new(writer, Compression::default());
        let mut writer = HashWriter {
            writer: e,
            hasher: Sha1::new(),
        };

        write!(writer, "{} {}\0", self.kind, self.expected_size)?;

        io::copy(&mut self.reader, &mut writer).context("stream file content to writer")?;

        writer.writer.finish()?;
        let hash = writer.hasher.finalize();
        Ok(hash.into())
    }
    pub(crate) fn write_to_objects(self) -> anyhow::Result<[u8; 20]> {
        // TODO: make the temporary file name unique with a timestamp or UUID
        let tmp = "temporary_file";
        let hash = self
            .write(std::fs::File::create(tmp).context("construct temporary file for object")?)
            .context("stream file into object")?;
        let hex_hash = hex::encode(hash);
        std::fs::create_dir_all(format!(".git/objects/{}/", &hex_hash[..2]))
            .context("create subdir of .git/objects")?;
        if let Err(e) = std::fs::rename(
            tmp,
            format!(".git/objects/{}/{}", &hex_hash[..2], &hex_hash[2..]),
        )
        .with_context(|| {
            format!(
                "rename temporary file to .git/objects/{}/{}",
                &hex_hash[..2],
                &hex_hash[2..]
            )
        }) {
            std::fs::remove_file(tmp)
                .with_context(|| format!("remove temporary file `{}`", tmp))?;
            return Err(e);
        }
        Ok(hash)
    }
}

struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
}

impl<W: Write> Write for HashWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
