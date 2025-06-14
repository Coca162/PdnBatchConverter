use std::{convert::identity, env::current_dir, path::PathBuf, sync::Arc};

use clap::Parser;
use color_eyre::eyre::{self, OptionExt};
use pdn_conv::{PdnHoster, VERSION};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[derive(Parser)]
#[command(display_name("PdnBatchConverter"), version(VERSION), about, long_about = None)]
enum Cli {
    Select {
        pdn_dll: PathBuf,
    },
    Convert {
        input: Vec<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli {
        Cli::Select { pdn_dll } => {
            pdn_conv::State::from_pdn_location(&pdn_dll)?;
        }
        Cli::Convert { input, output } => {
            let state = pdn_conv::State::init()?;

            let output = output.ok_or_else(current_dir).or_else(identity)?;

            rayon::ThreadPoolBuilder::new()
                .num_threads(state.parallelism().get())
                .build_global()?;

            fn convert_file(
                (output, state): &mut (PathBuf, Arc<PdnHoster>),
                input: PathBuf,
            ) -> eyre::Result<()> {
                output.push(input.file_name().ok_or_eyre("Invalid file name")?);
                output.set_extension("ora");
                state.ora_file_from_pdn(input, &output)?;
                output.pop();

                eyre::Result::Ok(())
            }

            input
                .into_par_iter()
                .try_for_each_with((output, state.pdn_hoster().clone()), convert_file)?;

            // state.pdn_hoster().ora_file_from_pdn(input, output)?;
        }
    }

    Ok(())
}
