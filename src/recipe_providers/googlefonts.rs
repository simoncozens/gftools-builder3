use babelfont::{Font, UserCoord};
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use serde_json::Value;
use std::{collections::HashMap, path::Path};

use crate::{
    error::ApplicationError,
    operations::ConfigOperationBuilder,
    recipe::{Provider, Recipe},
};

#[derive(PartialEq, Debug, Clone, Copy)]
enum Style {
    Roman,
    Italic,
}

pub type ItalicDescriptor = (String, UserCoord, UserCoord);

#[serde_inline_default]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GoogleFontsOptions {
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub outputs: HashMap<String, String>,
    #[serde(default)]
    pub extra: HashMap<String, Value>,

    // Path-related options
    #[serde_inline_default("../fonts/".to_string())]
    #[serde(rename = "outputDir")]
    pub output_dir: String,

    #[serde_inline_default("$outputDir/variable".to_string())]
    #[serde(rename = "vfDir")]
    pub vf_dir: String,

    #[serde_inline_default("$outputDir/ttf".to_string())]
    #[serde(rename = "ttDir")]
    pub tt_dir: String,

    #[serde_inline_default("$outputDir/otf".to_string())]
    #[serde(rename = "otDir")]
    pub ot_dir: String,

    #[serde_inline_default("$outputDir/woff".to_string())]
    #[serde(rename = "woffDir")]
    pub woff_dir: String,

    #[serde(default)]
    #[serde(rename = "filenameSuffix")]
    pub filename_suffix: Option<String>,

    // Options about what we build
    #[serde_inline_default(true)]
    #[serde(rename = "buildVariable")]
    pub build_variable: bool,

    #[serde_inline_default(true)]
    #[serde(rename = "buildStatic")]
    pub build_static: bool,

    // Oops, OTFs don't exist in the fontc universe
    #[serde_inline_default(true)]
    #[serde(rename = "buildTTF")]
    pub build_ttf: bool,

    #[serde_inline_default(true)]
    #[serde(rename = "buildWebfont")]
    pub build_webfont: bool,
}

impl Default for GoogleFontsOptions {
    fn default() -> Self {
        // Just deserialize nothing and let serde fill in the defaults
        serde_json::from_str("{}").unwrap()
    }
}

impl GoogleFontsOptions {
    fn vf_dir(&self) -> String {
        self.vf_dir.replace("$outputDir", &self.output_dir)
    }
    fn tt_dir(&self) -> String {
        self.tt_dir.replace("$outputDir", &self.output_dir)
    }
    fn ot_dir(&self) -> String {
        self.ot_dir.replace("$outputDir", &self.output_dir)
    }
    fn woff_dir(&self) -> String {
        self.woff_dir.replace("$outputDir", &self.output_dir)
    }

    pub fn vf_filename(
        &self,
        source: &Font,
        suffix: Option<&str>,
        extension: Option<&str>,
        italic_ds: Option<&ItalicDescriptor>,
        roman: Style,
    ) -> Result<String, ApplicationError> {
        let suffix = suffix.unwrap_or("");
        let extension = extension.unwrap_or("ttf");

        let mut sourcebase = source
            .source
            .as_ref()
            .and_then(|x| {
                Path::new(&x)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
            })
            .ok_or(ApplicationError::InvalidRecipe(
                "Source font does not have a valid filename".to_string(),
            ))?;

        let mut tags = source
            .axes
            .iter()
            .map(|axis| axis.tag.to_string())
            .collect::<Vec<String>>();
        if let Some((axis_tag, _, _)) = italic_ds.as_ref() {
            if roman == Style::Italic {
                sourcebase.push_str("-Italic");
            }
            tags.retain(|tag| tag != axis_tag);
        }

        tags.sort();
        let axis_tags = tags.join(",");

        let mut directory = self.vf_dir();
        if extension == "woff2" {
            directory = self.woff_dir();
        }

        if !suffix.is_empty() {
            if sourcebase.contains("-Italic") {
                sourcebase = sourcebase.replace("-Italic", &format!("{}-Italic", suffix));
            } else {
                sourcebase.push_str(suffix);
            }
        }

        Ok(format!("{directory}/{sourcebase}[{axis_tags}].{extension}"))
    }

    pub fn static_filename(
        &self,
        instance_filename: &str,
        suffix: Option<&str>,
        extension: Option<&str>,
    ) -> String {
        let suffix = suffix.unwrap_or("");
        let extension = extension.unwrap_or("ttf");

        let outdir = match extension {
            "ttf" => self.tt_dir(),
            "otf" => self.ot_dir(),
            "woff2" => self.woff_dir(),
            _ => self.tt_dir(),
        };

        let mut instancebase = Path::new(instance_filename)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| instance_filename.to_string());

        if !suffix.is_empty() {
            if let Some((familyname, style)) = instancebase.rsplit_once('-') {
                instancebase = format!("{familyname}{suffix}-{style}");
            } else {
                instancebase.push_str(suffix);
            }
        }

        format!("{outdir}/{instancebase}.{extension}")
    }
}

pub struct GoogleFontsProvider {
    options: GoogleFontsOptions,
    sources: Vec<Font>,
    recipe: Recipe,
}

impl GoogleFontsProvider {
    pub fn new(options: GoogleFontsOptions) -> Self {
        GoogleFontsProvider {
            options,
            sources: vec![],
            recipe: Recipe::default(),
        }
    }
}

impl Provider for GoogleFontsProvider {
    fn generate_recipe(mut self) -> Result<Recipe, ApplicationError> {
        self.load_all_sources()?;
        // Implementation for rewriting the recipe for Google fonts
        Ok(self.recipe)
    }
}

impl GoogleFontsProvider {
    fn load_all_sources(&mut self) -> Result<(), ApplicationError> {
        for source in &self.options.sources {
            let font = babelfont::load(source).map_err(|e| {
                ApplicationError::InvalidRecipe(format!("Failed to load source {source}: {e}"))
            })?;
            self.sources.push(font);
        }
        self.build_all_variables()?;
        Ok(())
    }

    fn build_all_variables(&mut self) -> Result<(), ApplicationError> {
        if !self.options.build_variable {
            return Ok(());
        }
        let new_recipes = self
            .sources
            .iter()
            .filter(|source| source.masters.len() >= 2)
            .flat_map(|source| {
                if let Some(italic_ds) = self.has_slant_italic(source) {
                    vec![
                        self.build_a_variable(source, Some(&italic_ds), Style::Italic),
                        self.build_a_variable(source, Some(&italic_ds), Style::Roman),
                        // if we have a stat file, we need to rewrite it here, unfortunately
                    ]
                } else {
                    vec![self.build_a_variable(source, None, Style::Roman)]
                }
            })
            .collect::<Result<Vec<Recipe>, ApplicationError>>()?;
        // Do STAT table
        // Do avar2
        for recipe in new_recipes {
            self.recipe.extend(recipe);
        }
        Ok(())
    }

    fn build_a_variable(
        &self,
        source: &Font,
        italic_ds: Option<&ItalicDescriptor>,
        roman: Style,
    ) -> Result<Recipe, ApplicationError> {
        log::debug!(
            "Considering how to build variable font for {}",
            source
                .names
                .family_name
                .get_default()
                .unwrap_or(&"Unknown family".to_string())
        );
        let mut recipe = Recipe::new();
        // Implementation for building a variable font
        let target = self.options.vf_filename(
            source,
            self.options.filename_suffix.as_deref(),
            Some("ttf"),
            italic_ds,
            roman,
        )?;
        log::debug!("VF target filename: {}", target);
        let mut builder = ConfigOperationBuilder::new();
        builder = builder.source(
            source
                .source
                .as_ref()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        );
        // Subset steps here
        builder = builder.compile(HashMap::new());
        // Any post-compile steps
        // Any VTT steps
        // If italic, subspace the axes according to style
        builder = builder.fix(HashMap::new());

        if self.options.build_webfont {
            let webfont_target = self.options.vf_filename(
                source,
                self.options.filename_suffix.as_deref(),
                Some("woff2"),
                italic_ds,
                roman,
            )?;
            log::debug!(" Building webfont target: {}", webfont_target);
            let webfont_builder = builder.clone().compress();
            recipe.insert(webfont_target, webfont_builder.build());
        }

        recipe.insert(target, builder.build());
        Ok(recipe)
    }

    fn has_slant_italic(&self, source: &Font) -> Option<ItalicDescriptor> {
        for axis in &source.axes {
            if axis.tag == "ital"
                && let Some((min, _, max)) = axis.bounds()
            {
                return Some((axis.tag.to_string(), min, max));
            }
        }

        for axis in &source.axes {
            if axis.tag == "slnt"
                && let Some((min, _, max)) = axis.bounds()
            {
                return Some((axis.tag.to_string(), max, min));
            }
        }

        None
    }
}
