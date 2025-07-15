use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::io;

pub(crate) fn invoke(write: bool, file: &PathBuf) -> anyhow::Result<()> {
    fn write_blob<W: Write>(path: &PathBuf, writer: W) -> anyhow::Result<String> {
        let stat =
            std::fs::metadata(path).with_context(|| format!("stat file `{}`", path.display()))?;

        let e = ZlibEncoder::new(writer, Compression::default());
        let mut writer = HashWriter {
            writer: e,
            hasher: Sha1::new(),
        };

        write!(writer, "blob {}\0", stat.len())?;

        let mut file =
            std::fs::File::open(path).with_context(|| format!("open file {}", path.display()))?;
        io::copy(&mut file, &mut writer).context("stream file content to writer")?;

        writer.writer.finish()?;
        let hash = writer.hasher.finalize();

        Ok(hex::encode(hash))
    }

    let hash = if write {
        let tmp = "temporary_blob";
        let hash = write_blob(
            &file,
            std::fs::File::create(tmp).context("construct temporary file for blob")?,
        )
        .context("write out blob object")?;

        std::fs::create_dir_all(format!(".git/objects/{}/", &hash[..2]))
            .context("create subdir of .git/objects")?;
        std::fs::rename(tmp, format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
            .context("move temporary file to .git/objects")?;

        hash
    } else {
        write_blob(&file, io::sink()).context("write out a blob object")?
    };

    println!("{hash}");
    Ok(())
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
