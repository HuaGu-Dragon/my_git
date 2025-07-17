use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use clap::Subcommand;

mod commands;
mod objects;

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
    LsTree {
        #[clap(long)]
        name_only: bool,

        tree_hash: String,
    },
    WriteTree,
    CommitTree {
        #[clap(short = 'm')]
        message: String,
        #[clap(short = 'p')]
        parent_hash: Option<String>,
        tree_hash: String,
    },
    Commit {
        #[clap(short = 'm')]
        message: String,
    },
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
        } => commands::cat_file::invoke(pretty_print, &object_hash)?,
        Command::HashObject { write, file } => commands::hash_object::invoke(write, &file)?,
        Command::LsTree {
            name_only,
            tree_hash,
        } => commands::ls_tree::invoke(name_only, &tree_hash)?,
        Command::WriteTree => commands::write_tree::invoke()?,
        Command::CommitTree {
            message,
            tree_hash,
            parent_hash,
        } => commands::commit_tree::invoke(message, &tree_hash, parent_hash.as_deref())?,
        Command::Commit { message } => {
            /***
             * .git/HEAD -> ref: refs/heads/main
             * .git/refs/heads/main -> parent_hash
             */

            let head = std::fs::read_to_string(".git/HEAD").context("read .git/HEAD")?;
            let Some(git_ref) = head.strip_prefix("ref: ") else {
                anyhow::bail!("invalid .git/HEAD format")
            };
            let git_ref = git_ref.trim();
            let parent_hash = std::fs::read_to_string(format!(".git/{git_ref}"))
                .with_context(|| format!("read current branch reference: `.git/{git_ref}`"))?;

            let parent_hash = parent_hash.trim();

            let Some(tree_hash) = commands::write_tree::write_tree_for(Path::new("./"))? else {
                eprint!("empty tree");
                return Ok(());
            };

            let tree_hash = hex::encode(tree_hash);

            let commit_hash =
                commands::commit_tree::write_commit(message, &tree_hash, Some(parent_hash))
                    .context("write commit")?;
            let commit_hash = format!("{}\n", hex::encode(commit_hash));

            std::fs::write(format!(".git/{git_ref}"), &commit_hash)
                .with_context(|| format!("update commit hash to `.git/{git_ref}`"))?;

            print!("HEAD is now at {commit_hash}");
        }
    }

    Ok(())
}
