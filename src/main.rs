mod error;
mod operations;
mod orchestrator;
mod recipe;

use std::{process::exit, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncWriteExt, stderr},
    time::sleep,
};

use crate::{
    operations::{
        Operation, SourceSink, buildstatic::BuildStatic, buildvariable::BuildVariable,
        glyphs2ufo::Glyphs2UFO,
    },
    orchestrator::{Configuration, Context},
};

use petgraph::{dot::Dot, graph::Graph};

#[tokio::main]
async fn main() {
    let job_limit = num_cpus::get();

    let mut g = Graph::<Arc<Box<dyn Operation>>, String>::new();
    let source: Box<dyn Operation> = Box::new(SourceSink::Source);
    let glyphs2ufo: Box<dyn Operation> = Box::new(Glyphs2UFO::new());
    let compile: Box<dyn Operation> = Box::new(BuildVariable::new());
    let variable_ttf: Box<dyn Operation> = Box::new(SourceSink::Sink);
    let source_node = g.add_node(source.into());

    let compile_node = g.add_node(compile.into());
    let variable_ttf_node = g.add_node(variable_ttf.into());

    let g2u_node = g.add_node(glyphs2ufo.into());
    g.add_edge(source_node, g2u_node, "Nunito.glyphs".to_string());
    g.add_edge(source_node, compile_node, "Nunito.glyphs".to_string());
    g.add_edge(
        compile_node,
        variable_ttf_node,
        "../fonts/variable/Nunito[wght].ttf".to_string(),
    );
    for instance in ([
        "Bold",
        "BoldItalic",
        "Heavy",
        "HeavyItalic",
        "ExtraLight",
        "ExtraLightItalic",
    ]) {
        let build_instance: Box<dyn Operation> = Box::new(BuildStatic::new());
        let instance_ttf: Box<dyn Operation> = Box::new(SourceSink::Sink);
        let instance_node = g.add_node(build_instance.into());
        let instance_ttf_node = g.add_node(instance_ttf.into());
        g.add_edge(
            g2u_node,
            instance_node,
            format!("instance_ufo/Nunito-{instance}.ufo"),
        );
        g.add_edge(
            instance_node,
            instance_ttf_node,
            format!("../fonts/ttf/Nunito-{instance}.ttf"),
        );
    }

    println!("{:?}", Dot::new(&g));

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
