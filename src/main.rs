mod error;
mod operations;
mod orchestrator;
mod recipe;

use std::{process::exit, sync::Arc, time::Duration, vec};
use tokio::{
    io::{AsyncWriteExt, stderr},
    time::sleep,
};

use crate::{
    operations::{
        Operation, buildstatic::BuildStatic, buildvariable::BuildVariable, glyphs2ufo::Glyphs2UFO,
    },
    orchestrator::{Configuration, Context},
};

use petgraph::{
    Direction::{self},
    graph::Graph,
};

#[tokio::main]
async fn main() {
    let job_limit = num_cpus::get();

    let mut g = Graph::<Arc<Box<dyn Operation>>, String>::new();
    // let source = g.add_node(Box::new(SourceLoader::new()));
    let glyphs2ufo: Box<dyn Operation> = Box::new(Glyphs2UFO::new(
        "Nunito.glyphs".to_string(),
        vec![
            "master_ufo/Nunito.designspace".to_string(),
            "master_ufo/Nunito-Heavy.ufo".to_string(),
            "master_ufo/Nunito-Bold.ufo".to_string(),
            "master_ufo/Nunito-HeavyItalic.ufo".to_string(),
            "master_ufo/Nunito-ExtraLightItalic.ufo".to_string(),
            "master_ufo/Nunito-ExtraLight.ufo".to_string(),
            "master_ufo/Nunito-BoldItalic.ufo".to_string(),
        ],
    ));
    let compile: Box<dyn Operation> = Box::new(BuildVariable::new(
        "master_ufo/Nunito.designspace".to_string(),
        "../fonts/variable/Nunito[ital,wght].ttf".to_string(),
    ));
    let g2u_node = g.add_node(glyphs2ufo.into());
    let compile_node = g.add_node(compile.into());
    g.add_edge(
        g2u_node,
        compile_node,
        "master_ufo/Nunito.designspace".to_string(),
    );
    let build_heavy: Box<dyn Operation> = Box::new(BuildStatic::new(
        "master_ufo/Nunito-Heavy.ufo".to_string(),
        "../fonts/ttf/Nunito-Heavy.ttf".to_string(),
    ));
    let heavy_node = g.add_node(build_heavy.into());
    g.add_edge(
        g2u_node,
        heavy_node,
        "master_ufo/Nunito-Heavy.ufo".to_string(),
    );

    let build_bold: Box<dyn Operation> = Box::new(BuildStatic::new(
        "master_ufo/Nunito-Bold.ufo".to_string(),
        "../fonts/ttf/Nunito-Bold.ttf".to_string(),
    ));
    let bold_node = g.add_node(build_bold.into());
    g.add_edge(
        g2u_node,
        bold_node,
        "master_ufo/Nunito-Bold.ufo".to_string(),
    );

    println!("Externals:");
    println!(
        "In: {:?}",
        g.externals(Direction::Incoming)
            .map(|nx| g.node_weight(nx).map(|n| n.shortname()))
            .collect::<Vec<_>>()
    );
    println!(
        "Out: {:?}",
        g.externals(Direction::Outgoing)
            .map(|nx| g.node_weight(nx).map(|n| n.shortname()))
            .collect::<Vec<_>>()
    );

    let configuration = Configuration::new(g);

    let context = Arc::new(Context::new(job_limit, Arc::new(configuration)));
    if let Err(error) = orchestrator::run(&context).await {
        stderr()
            .write_all(format!("{error}\n").as_bytes())
            .await
            .unwrap();

        // Delay for the error message to be written completely hopefully.
        sleep(Duration::from_millis(1)).await;

        exit(1)
    }
}
