mod context;
mod error;
mod ir;
mod operations;
mod run;

use context::Context;
use std::{collections::HashSet, process::exit, sync::Arc, time::Duration, vec};
use tokio::{
    io::{AsyncWriteExt, stderr},
    time::sleep,
};

use crate::ir::{Build, Configuration, Rule};

#[tokio::main]
async fn main() {
    let job_limit = num_cpus::get();
    let fontmake_ufo = Build::new(
        vec![
            Arc::from("master_ufo/Nunito.designspace"),
            Arc::from("master_ufo/Nunito-Heavy.ufo"),
            Arc::from("master_ufo/Nunito-Bold.ufo"),
            Arc::from("master_ufo/Nunito-HeavyItalic.ufo"),
            Arc::from("master_ufo/Nunito-ExtraLightItalic.ufo"),
            Arc::from("master_ufo/Nunito-ExtraLight.ufo"),
            Arc::from("master_ufo/Nunito-BoldItalic.ufo"),
        ],
        Some(Rule::new(
            "fontmake -o ufo --instance-dir instance_ufo -g Nunito.glyphs",
            Some("Convert glyphs file to UFO".to_string()),
        )),
        vec![],
    );
    let build_vf = Build::new(
        vec![Arc::from("../fonts/variable/Nunito[ital,wght].ttf")],
        Some(Rule::new(
            "fontmake -o variable -m master_ufo/Nunito.designspace --filter ... --filter FlattenComponentsFilter --filter DecomposeTransformedComponentsFilter --output-path ../fonts/variable/Nunito[ital,wght].ttf",
            Some("Build a variable font from Designspace".to_string()),
        )),
        vec![fontmake_ufo.id().into()],
    );

    let gen_stat = Build::new(
        vec![Arc::from(
            "../fonts/variable/Nunito[ital,wght].ttf.statstamp",
        )],
        Some(Rule::new(
            "gftools-gen-stat --inplace  -- ../fonts/variable/Nunito[ital,wght].ttf  && touch ../fonts/variable/Nunito[ital,wght].ttf.statstamp",
            Some("Add a STAT table to a set of variable fonts".to_string()),
        )),
        vec![build_vf.id().into()],
    );
    let fix = Build::new(
        vec![Arc::from(
            "../fonts/variable/Nunito[ital,wght].ttf.fixstamp",
        )],
        Some(Rule::new(
            "gftools-fix-font -o ../fonts/variable/Nunito[ital,wght].ttf  ../fonts/variable/Nunito[ital,wght].ttf && touch ../fonts/variable/Nunito[ital,wght].ttf.fixstamp",
            Some("Run the font fixer in-place and touch a stamp file".to_string()),
        )),
        vec![build_vf.id().into(), gen_stat.id().into()],
    );

    let default_outputs = HashSet::from_iter([&fix].iter().map(|s| s.id().into()));
    let mut configuration = Configuration::new(default_outputs, None);
    configuration.add_job(build_vf.into());
    configuration.add_job(fontmake_ufo.into());
    configuration.add_job(gen_stat.into());
    configuration.add_job(fix.into());

    let context = Arc::new(Context::new(job_limit, Arc::new(configuration.clone())));
    if let Err(error) = run::run(&context).await {
        stderr()
            .write_all(format!("{error}\n").as_bytes())
            .await
            .unwrap();

        // Delay for the error message to be written completely hopefully.
        sleep(Duration::from_millis(1)).await;

        exit(1)
    }
}
