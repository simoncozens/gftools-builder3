mod error;
mod graph;
mod operations;
mod orchestrator;
mod recipe;

use clap::{ArgAction, Parser};
use std::{process::exit, time::Duration};
use tokio::{
    io::{AsyncWriteExt, stderr},
    time::sleep,
};

#[derive(clap::Parser)]
struct Args {
        /// Increase logging
    #[clap(short, long, action = ArgAction::Count, help_heading = "Logging")]
    pub verbose: u8,
    /// Draw the graph of the build process
    /// This will create a file named `graph.svg` in the current directory
    #[clap(long)]
    graph: bool,
    config_file: String,
}

#[tokio::main]
async fn main() {
    let job_limit = num_cpus::get();
    let args = Args::parse();
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Error)
        .filter_module("gftools_builder::orchestrator", match args.verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            _ => log::LevelFilter::Debug,
        })
        .format_timestamp(Some(env_logger::TimestampPrecision::Seconds))
        .format_module_path(false)
        .format_target(false)
        .init();
    let config_yaml = std::fs::read_to_string(&args.config_file).unwrap_or_else(|e| {
        log::error!("Could not read config file {}: {e}", args.config_file);
        exit(1)
    });
    let mut config = serde_yaml_ng::from_str::<recipe::Config>(&config_yaml).unwrap_or_else(|e| {
        log::error!("Could not parse config file {}: {e}", args.config_file);
        exit(1)
    });
    let g = config
        .to_graph()
        .unwrap_or_else(|_| panic!("Could not convert config to graph: {}", args.config_file));
    g.ensure_directories().unwrap_or_else(|_| {
        log::error!(
            "Could not ensure directories for graph: {}",
            args.config_file
        );
        exit(1)
    });

    if args.graph {
        let graph = g
            .draw()
            .unwrap_or_else(|_| panic!("Could not draw graph: {}", args.config_file));
        std::fs::write("graph.svg", graph)
            .unwrap_or_else(|_| panic!("Could not write graph to file: graph.svg"));
    }

    if let Err(error) = orchestrator::run(g, job_limit).await {
        stderr()
            .write_all(format!("{error}\n").as_bytes())
            .await
            .unwrap();

        // Delay for the error message to be written completely hopefully.
        sleep(Duration::from_millis(1)).await;

        exit(1)
    }
}
