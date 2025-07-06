use std::{convert::identity, env::current_dir, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::{self, OptionExt};
use pdn_conv::{ConversionFormat, VERSION};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[derive(Parser)]
#[command(display_name("PdnBatchConverter"), version(VERSION), about, long_about = None)]
enum Cli {
    Select {
        pdn_dll: PathBuf,
    },
    #[command(subcommand)]
    Convert(Convert),
}

#[derive(Subcommand)]
#[command(about = "Convert .pdn files to the specified format", long_about = None)]
enum Convert {
    #[command(visible_alias = "jpg")]
    Jpeg {
        #[arg(short, long, default_value_t = 95)]
        quality: u8,
        #[command(flatten)]
        args: ConvertArgs,
    },
    Png(ConvertArgs),
    Ora(ConvertArgs),
}

#[derive(Args, Debug, Clone)]
struct ConvertArgs {
    input: Vec<PathBuf>,
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli {
        Cli::Select { pdn_dll } => {
            pdn_conv::State::from_pdn_location(&pdn_dll)?;
        }
        Cli::Convert(format) => {
            let (format, ConvertArgs { input, output }) = match format {
                Convert::Jpeg { quality, args } => (ConversionFormat::Jpeg { quality }, args),
                Convert::Ora(args) => (ConversionFormat::Ora, args),
                Convert::Png(args) => (ConversionFormat::Png, args),
            };

            let state = pdn_conv::State::init()?;

            let output = output.ok_or_else(current_dir).or_else(identity)?;

            rayon::ThreadPoolBuilder::new()
                .num_threads(state.parallelism().get())
                .build_global()?;

            input.into_par_iter().try_for_each_with(
                (output, state.pdn_hoster().clone()),
                |(output, state), input| -> eyre::Result<()> {
                    output.push(input.file_name().ok_or_eyre("Invalid file name")?);
                    output.set_extension(format.ext());
                    let res = state.file_from_pdn(format, &input, output);
                    output.pop();
                    res
                },
            )?;
        }
    }

    Ok(())
}
