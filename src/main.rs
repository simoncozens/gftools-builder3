mod buildsystem;
mod error;
mod operations;
mod recipe;
mod recipe_providers;

use tracing_chrome::ChromeLayerBuilder;

use clap::{ArgAction, Parser};
use std::{process::exit, time::Duration};
use tokio::{
    io::{AsyncWriteExt, stderr},
    time::sleep,
};
use tracing_subscriber::{EnvFilter, prelude::*};

#[derive(clap::Parser)]
struct Args {
    /// Increase logging
    #[clap(short, long, action = ArgAction::Count, help_heading = "Logging")]
    pub verbose: u8,
    /// Generate the recipe and dump as YAML but do not build
    #[clap(long)]
    pub generate: bool,
    /// Enable profiling and write trace data to the specified file
    #[clap(long)]
    pub profile: Option<String>,
    #[cfg(feature = "graphviz")]
    /// Draw the graph of the build process
    /// This will create a file named `graph.svg` in the current directory
    #[clap(long)]
    graph: bool,
    #[clap(long)]
    ascii_graph: bool,
    config_file: String,
}

#[tokio::main]
async fn main() {
    let job_limit = num_cpus::get();
    let args = Args::parse();
    let mut _guard = None;
    if let Some(ref profile_file) = args.profile {
        // Initialize tracing subscriber if profiling is enabled
        let (chrome_layer, guard) = ChromeLayerBuilder::new()
            .include_args(true)
            .include_locations(true)
            .file(profile_file)
            .build();
        _guard = Some(guard);

        let env_filter = EnvFilter::new("gftools_builder=info");

        // Set up the tracing subscriber with JSON output to stderr
        tracing_subscriber::registry()
            .with(env_filter)
            .with(chrome_layer)
            .init();
    }

    log::info!("Starting gftools-builder with {} parallel jobs", job_limit);
    let config_yaml = std::fs::read_to_string(&args.config_file).unwrap_or_else(|e| {
        log::error!("Could not read config file {}: {e}", args.config_file);
        exit(1)
    });
    // Parse the YAML into a recipe config file
    let config = serde_yaml_ng::from_str::<recipe::Config>(&config_yaml).unwrap_or_else(|e| {
        log::error!("Could not parse config file {}: {e}", args.config_file);
        exit(1)
    });

    // Change to the config file's directory
    if let Some(config_dir) = std::path::Path::new(&args.config_file).parent() {
        std::env::set_current_dir(config_dir).unwrap_or_else(|e| {
            log::error!(
                "Could not change directory to config file's directory {}: {}",
                config_dir.display(),
                e
            );
            exit(1)
        });
    }

    if args.generate {
        let recipe = config.recipe().unwrap_or_else(|e| {
            panic!(
                "Could not convert config {} to recipe: {}",
                args.config_file, e
            )
        });
        let recipe_yaml = serde_yaml_ng::to_string(&recipe)
            .unwrap_or_else(|_| panic!("Could not serialize recipe to YAML: {}", args.config_file));
        println!("{recipe_yaml}");
        return;
    }
    // Use the recipe to create a build graph
    let g = config
        .to_graph()
        .unwrap_or_else(|_| panic!("Could not convert config to graph: {}", args.config_file));
    g.ensure_directories().unwrap_or_else(|e| {
        log::error!(
            "Could not ensure directories for graph: {}: {}",
            args.config_file,
            e
        );
        exit(1)
    });

    #[cfg(feature = "graphviz")]
    if args.graph {
        let graph = g
            .draw()
            .unwrap_or_else(|_| panic!("Could not draw graph: {}", args.config_file));
        std::fs::write("graph.svg", graph)
            .unwrap_or_else(|_| panic!("Could not write graph to file: graph.svg"));
        println!("Wrote build graph to {}/graph.svg", std::env::current_dir().unwrap().display());
        return;
    }

    if args.ascii_graph {
        let graph = g
            .ascii()
            .unwrap_or_else(|_| panic!("Could not create ASCII graph: {}", args.config_file));
        println!("{graph}");
        return;
    }

    if let Err(error) = buildsystem::run(g, job_limit).await {
        stderr()
            .write_all(format!("{error}\n").as_bytes())
            .await
            .unwrap();

        // Delay for the error message to be written completely hopefully.
        sleep(Duration::from_millis(1)).await;

        exit(1)
    }
}
