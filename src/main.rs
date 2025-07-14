#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
#[allow(unused_imports)]
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::bail;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::io;

use clap::Parser;
use clap::Subcommand;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write: bool,

        file: PathBuf,
    },
}

enum Kind {
    Blob,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    match args.command {
        Command::Init => {
            std::fs::create_dir(".git").unwrap();
            std::fs::create_dir(".git/objects").unwrap();
            std::fs::create_dir(".git/refs").unwrap();
            std::fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory");
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
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
        }
        Command::HashObject { write, file } => {
            fn write_blob<W: Write>(path: &PathBuf, writer: W) -> anyhow::Result<String> {
                let stat = std::fs::metadata(path)
                    .with_context(|| format!("stat file `{}`", path.display()))?;

                let e = ZlibEncoder::new(writer, Compression::default());
                let mut writer = HashWriter {
                    writer: e,
                    hasher: Sha1::new(),
                };

                write!(writer, "blob {}\0", stat.len())?;

                let mut file = std::fs::File::open(path)
                    .with_context(|| format!("open file {}", path.display()))?;
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

            println!("{hash}")
        }
    }

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
