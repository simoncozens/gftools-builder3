use std::{os::unix::process::ExitStatusExt, process::Output};

use crate::{
    buildsystem::{DataKind, Operation, OperationOutput},
    error::ApplicationError,
};
use babelfont::Font;
use fontmerge::fontmerge;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AddSubsetConfig {
    pub include_glyphs: Vec<String>,
    pub exclude_glyphs: Vec<String>,
    pub include_codepoints: Vec<char>,
    #[serde(
        default,
        serialize_with = "existing_glyph_handling_ser",
        deserialize_with = "existing_glyph_handling_deser"
    )]
    pub existing_glyph_handling: fontmerge::ExistingGlyphHandling,
    #[serde(
        default,
        serialize_with = "layout_handling_ser",
        deserialize_with = "layout_handling_deser"
    )]
    pub layout_handling: fontmerge::LayoutHandling,
}

fn existing_glyph_handling_ser<S>(
    existing_glyph_handling: &fontmerge::ExistingGlyphHandling,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(match existing_glyph_handling {
        fontmerge::ExistingGlyphHandling::Skip => "skip",
        fontmerge::ExistingGlyphHandling::Replace => "replace",
    })
}

fn existing_glyph_handling_deser<'de, D>(
    deserializer: D,
) -> Result<fontmerge::ExistingGlyphHandling, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    match s {
        "skip" => Ok(fontmerge::ExistingGlyphHandling::Skip),
        "replace" => Ok(fontmerge::ExistingGlyphHandling::Replace),
        _ => Err(serde::de::Error::custom(format!(
            "Invalid existing_glyph_handling value: {}",
            s
        ))),
    }
}

fn layout_handling_ser<S>(
    layout_handling: &fontmerge::LayoutHandling,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(match layout_handling {
        fontmerge::LayoutHandling::Subset => "subset",
        fontmerge::LayoutHandling::Closure => "closure",
        fontmerge::LayoutHandling::Ignore => "ignore",
    })
}

fn layout_handling_deser<'de, D>(deserializer: D) -> Result<fontmerge::LayoutHandling, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    match s {
        "subset" => Ok(fontmerge::LayoutHandling::Subset),
        "closure" => Ok(fontmerge::LayoutHandling::Closure),
        "ignore" => Ok(fontmerge::LayoutHandling::Ignore),
        _ => Err(serde::de::Error::custom(format!(
            "Invalid layout_handling value: {}",
            s
        ))),
    }
}

#[derive(PartialEq, Debug)]
pub(crate) struct AddSubset {
    config: AddSubsetConfig,
}

impl AddSubset {
    pub fn new() -> Self {
        AddSubset {
            config: AddSubsetConfig {
                include_glyphs: vec![],
                exclude_glyphs: vec![],
                include_codepoints: vec![],
                existing_glyph_handling: fontmerge::ExistingGlyphHandling::Skip,
                layout_handling: fontmerge::LayoutHandling::Subset,
            },
        }
    }

    pub fn glyphset_filter(&self, font1: &mut Font, font2: &Font) -> fontmerge::GlyphsetFilter {
        fontmerge::GlyphsetFilter::new(
            self.config
                .include_glyphs
                .iter()
                .map(|x| x.into())
                .collect(),
            self.config
                .exclude_glyphs
                .iter()
                .map(|x| x.into())
                .collect(),
            self.config.include_codepoints.clone(),
            font1,
            &font2,
            self.config.existing_glyph_handling,
        )
    }
}

impl Operation for AddSubset {
    fn shortname(&self) -> &str {
        "AddSubset"
    }

    fn input_kinds(&self) -> Vec<DataKind> {
        vec![DataKind::SourceFont]
    }

    fn output_kinds(&self) -> Vec<DataKind> {
        vec![DataKind::SourceFont]
    }

    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        // Assert that we have two inputs
        let mut input_font = inputs
            .first()
            .ok_or_else(|| ApplicationError::WrongInputs("No input".into()))?
            .to_font_source()?;
        let donor_font = inputs
            .get(1)
            .ok_or_else(|| ApplicationError::WrongInputs("No donor font".into()))?
            .to_font_source()?;
        let filter = self.glyphset_filter(&mut *input_font, &*donor_font);
        let output_font = fontmerge(
            *input_font,
            *donor_font,
            filter,
            self.config.layout_handling,
        )
        .map_err(|e| ApplicationError::Other(format!("Font merge failed: {}", e)))?;
        outputs[0].set_font_source(Box::new(output_font))?;
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: vec![],
            stderr: vec![],
        })
    }

    fn description(&self) -> String {
        "Merge subset into font".to_string()
    }
}
