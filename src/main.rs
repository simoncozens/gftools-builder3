mod error;
mod operations;
mod orchestrator;
mod recipe;

use std::{collections::HashSet, process::exit, sync::Arc, time::Duration, vec};
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

#[tokio::main]
async fn main() {
    let job_limit = num_cpus::get();
    let fontmake_ufo = Glyphs2UFO {
        source: "Nunito.glyphs".to_string(),
        outputs: vec![
            "master_ufo/Nunito.designspace".to_string(),
            "master_ufo/Nunito-Heavy.ufo".to_string(),
            "master_ufo/Nunito-Bold.ufo".to_string(),
            "master_ufo/Nunito-HeavyItalic.ufo".to_string(),
            "master_ufo/Nunito-ExtraLightItalic.ufo".to_string(),
            "master_ufo/Nunito-ExtraLight.ufo".to_string(),
            "master_ufo/Nunito-BoldItalic.ufo".to_string(),
        ],
        dependencies: vec![],
    };
    let build_vf = BuildVariable {
        source: "master_ufo/Nunito.designspace".to_string(),
        output: "../fonts/variable/Nunito[ital,wght].ttf".to_string(),
        dependencies: vec![fontmake_ufo.id().into()],
    };
    let build_heavy = BuildStatic {
        source: "master_ufo/Nunito-Heavy.ufo".to_string(),
        output: "../fonts/ttf/Nunito-Heavy.ttf".to_string(),
        dependencies: vec![fontmake_ufo.id().into()],
    };
    let build_bold = BuildStatic {
        source: "master_ufo/Nunito-Bold.ufo".to_string(),
        output: "../fonts/ttf/Nunito-Bold.ttf".to_string(),
        dependencies: vec![fontmake_ufo.id().into()],
    };

    let top_levels: Vec<Box<&dyn Operation>> = vec![
        Box::new(&build_vf),
        Box::new(&build_heavy),
        Box::new(&build_bold),
    ];

    let default_outputs = HashSet::from_iter(top_levels.iter().map(|s| s.id().into()));
    let mut configuration = Configuration::new(default_outputs, None);
    configuration.add_job(Box::new(build_vf));
    configuration.add_job(Box::new(build_heavy));
    configuration.add_job(Box::new(build_bold));
    configuration.add_job(Box::new(fontmake_ufo));
    // configuration.add_job(gen_stat.into());
    // configuration.add_job(fix.into());

    let context = Arc::new(Context::new(job_limit, Arc::new(configuration.clone())));
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
