mod error;
mod graph;
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
        fontc::Fontc, fix::Fix, glyphs2ufo::Glyphs2UFO, SourceSink
    },
    graph::BuildGraph,
};

#[tokio::main]
async fn main() {
    let job_limit = num_cpus::get();

    let mut g = BuildGraph::new();
    g.add_path("Nunito.glyphs", vec![
        (Arc::new(Box::new(Fontc)), None),
        (Arc::new(Box::new(Fix)), None),
    ], "../fonts/variable/Nunito[wght].ttf");
    for instance in [
        "Black",
        "BlackItalic",
        "Bold",
        "BoldItalic",
        "ExtraBold",
        "ExtraBoldItalic",
        "ExtraLight",
        "ExtraLightItalic",
        "Italic",
        "Light",
        "LightItalic",
        "Medium",
        "MediumItalic",
        "SemiBold",
        "SemiBoldItalic",
    ] {
        g.add_path("Nunito.glyphs", vec![
            (Arc::new(Box::new(Glyphs2UFO)), None),
            (Arc::new(Box::new(Fontc)), Some(&format!("instance_ufo/Nunito-{instance}.ufo"))),
        ], format!("../fonts/ttf/Nunito-{instance}.ttf").as_str());
    }

    println!("{}", g.draw());


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
