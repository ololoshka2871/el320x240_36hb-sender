use structopt::StructOpt;

use el320x240_36hb_sender::run;

fn main() {
    let args = el320x240_36hb_sender::args::Cli::from_args();

    pollster::block_on(run(args));
}
