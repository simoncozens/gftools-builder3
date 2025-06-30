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
        buildstatic::BuildStatic, buildvariable::BuildVariable, fix::Fix, glyphs2ufo::Glyphs2UFO, Operation, RawOperationOutput, SourceSink
    },
    orchestrator::{BuildGraph, Configuration, Context},
};

use petgraph::dot::Dot;

#[tokio::main]
async fn main() {
    let job_limit = num_cpus::get();

    let mut g = BuildGraph::new();
    let source: Box<dyn Operation> = Box::new(SourceSink::Source);
    let glyphs2ufo: Box<dyn Operation> = Box::new(Glyphs2UFO);
    let compile: Box<dyn Operation> = Box::new(BuildVariable);
    let fix: Box<dyn Operation> = Box::new(Fix);
    let variable_ttf: Box<dyn Operation> = Box::new(SourceSink::Sink);
    let source_node = g.add_node(source.into());

    let compile_node = g.add_node(compile.into());
    let fix_node = g.add_node(fix.into());
    let variable_ttf_node = g.add_node(variable_ttf.into());

    // let g2u_node = g.add_node(glyphs2ufo.into());
    // g.add_edge(source_node, g2u_node, RawOperationOutput::from("Nunito.glyphs").into());
    g.add_edge(source_node, compile_node, RawOperationOutput::from("Nunito.glyphs").into());
    g.add_edge(compile_node, fix_node, RawOperationOutput::TemporaryFile(None).into());
    g.add_edge(
        fix_node,
        variable_ttf_node,
       RawOperationOutput::from( "../fonts/variable/Nunito[wght].ttf").into(),
    );
    // for instance in [
    //     "Black",
    //     "BlackItalic",
    //     "Bold",
    //     "BoldItalic",
    //     "ExtraBold",
    //     "ExtraBoldItalic",
    //     "ExtraLight",
    //     "ExtraLightItalic",
    //     "Italic",
    //     "Light",
    //     "LightItalic",
    //     "Medium",
    //     "MediumItalic",
    //     "SemiBold",
    //     "SemiBoldItalic",
    // ] {
    //     let build_instance: Box<dyn Operation> = Box::new(BuildStatic);
    //     let instance_ttf: Box<dyn Operation> = Box::new(SourceSink::Sink);
    //     let instance_node = g.add_node(build_instance.into());
    //     let instance_ttf_node = g.add_node(instance_ttf.into());
    //     g.add_edge(
    //         g2u_node,
    //         instance_node,
    //         RawOperationOutput::from(format!("instance_ufo/Nunito-{instance}.ufo").as_str()).into(),
    //     );
    //     g.add_edge(
    //         instance_node,
    //         instance_ttf_node,
    //         RawOperationOutput::from(format!("../fonts/ttf/Nunito-{instance}.ttf").as_str()).into(),
    //     );
    // }

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
