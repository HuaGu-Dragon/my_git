#[allow(unused_imports)]
use std::env;
use std::ffi::CStr;
#[allow(unused_imports)]
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;

use anyhow::Context;
use anyhow::bail;
use flate2::read::ZlibDecoder;
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
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
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
            let f = fs::File::open(format!(
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
                .parse::<usize>()
                .with_context(|| format!(".git/objects file header has invalid size: {size}"))?;

            buf.clear();
            buf.resize(size, 0);
            z.read_exact(&mut buf)
                .context("read object content from .git/objects")?;
            let n = z
                .read(&mut [0])
                .context("read object content from .git/objects")?;
            if n != 0 {
                bail!("expected end of object content, got more data");
            }

            let stdio = io::stdout();
            let mut stdout = stdio.lock();
            match kind {
                Kind::Blob => stdout
                    .write_all(&buf)
                    .context("write blob object content to stdout")?,
            };
        }
    }

    Ok(())

    // Uncomment this block to pass the first stage
    // let args: Vec<String> = env::args().collect();
    // if args[1] == "init" {
    //     fs::create_dir(".git").unwrap();
    //     fs::create_dir(".git/objects").unwrap();
    //     fs::create_dir(".git/refs").unwrap();
    //     fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
    //     println!("Initialized git directory")
    // } else {
    //     println!("unknown command: {}", args[1])
    // }
}
