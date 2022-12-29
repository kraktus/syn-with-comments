use std::{fs::File, io::Read, path::PathBuf};

use clap::{ArgAction, Parser};
use env_logger::Builder;
use log::LevelFilter;
use pretty_assertions::StrComparison;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[arg(help = "target, if a dir will loop for all .rs files")]
    target: PathBuf,
    #[arg(short, long, action = ArgAction::Count, default_value_t = 2)]
    verbose: u8,
}

fn main() {
    #[cfg(feature = "dhat")]
    let _profiler = dhat::Profiler::new_heap();
    let args = Cli::parse();
    let mut builder = Builder::new();
    builder
        .filter(
            None,
            match args.verbose {
                0 => LevelFilter::Error,
                1 => LevelFilter::Info,
                2 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            },
        )
        .default_format();

    builder.init();
    let entries: Vec<_> = args
        .target
        .read_dir()
        .map(|entries| {
            entries
                .filter_map(|entry_res| {
                    let entry = entry_res.unwrap();
                    (entry.file_type().expect("file_type failed").is_file()
                        && entry
                            .path()
                            .file_name()
                            .map(|n| n.to_string_lossy())
                            .map_or(true, |n| {
                                let file_name = n.to_string();
                                let extension = file_name.split_once('.').unwrap().1;
                                extension == "rs"
                            }))
                    .then(|| entry.path())
                })
                .collect()
        })
        .unwrap_or_else(|_| vec![args.target]);
    for file_path in entries.into_iter() {
        let mut file = File::open(&file_path).expect("reading source code failed");
        let mut src = String::new();
        file.read_to_string(&mut src).expect("Unable to read file");
        let parsed_back_and_forth = prettyplease::unparse(
            &syn::parse2(syn_with_comments::parse_str(&src).expect("Parsing failed"))
                .expect("Parsing to File failed"),
        );
        println!("diff of {file_path:?}");
        print!("{}", StrComparison::new(&src, &parsed_back_and_forth));
        println!("Continue? press `q` to exit");
        let mut continue_ = String::new();
        let _ = std::io::stdin().read_line(&mut continue_).unwrap();
        if continue_ == "q" {
            break;
        }
    }
}
