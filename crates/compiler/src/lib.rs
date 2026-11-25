//! Rust compiler for CSX.

mod parse;

use parse::{ClassBlock, Condition, Declaration, Layer};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

#[derive(Tsify, Serialize, Deserialize)]
#[tsify(rename = "CompilerOptions")]
pub struct JsCompilerOptions {
    /// Implementation to resolve an import relative to its importer into a canonical module ID
    #[serde(with = "serde_wasm_bindgen::preserve")]
    #[tsify(type = "(specifier: string, importer: string) => Promise<string>")]
    pub resolve: js_sys::Function,

    /// Implementaion to load CSS source text for a canonical module ID
    #[serde(with = "serde_wasm_bindgen::preserve")]
    #[tsify(type = "(id: string) => Promise<string>")]
    pub load: js_sys::Function,
}

#[derive(Tsify, Serialize, Deserialize)]
#[tsify(rename = "CompileModuleOptions")]
pub struct JsCompileModuleOptions {
    /// The module ID we're compiling
    pub id: String,
    /// The modules source
    pub source: String,
}

#[derive(Default, Tsify, Serialize, Deserialize)]
#[tsify(hashmap_as_object, rename = "CompileModuleResult")]
pub struct JsCompileModuleResult {
    /// User class name -> property key -> generated atomic class name.
    /// Conditional keys include the media query and selector suffix when present.
    pub exports: HashMap<String, HashMap<String, String>>,
    /// Export name -> preferred rule start (zero-based line and UTF-16 column).
    #[serde(rename = "exportLocations")]
    pub export_locations: HashMap<String, ExportLocation>,
    /// Canonical IDs of modules this compilation depends on.
    pub dependencies: Vec<String>,
    #[tsify(type = "Diagnostic[]")]
    pub diagnostics: Vec<JsDiagnostic>,
}

#[derive(Tsify, Serialize, Deserialize)]
pub struct ExportLocation {
    pub line: u32,
    pub column: u32,
}

#[derive(Tsify, Serialize, Deserialize)]
#[tsify(rename = "Diagnostic")]
pub struct JsDiagnostic {
    /// Canonical ID of the module responsible for the diagnostic.
    pub id: String,
    #[tsify(type = "DiagnosticSeverity")]
    pub severity: JsDiagnosticSeverity,
    pub message: String,
}

#[derive(Tsify, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[tsify(rename = "DiagnosticSeverity")]
pub enum JsDiagnosticSeverity {
    Error,
    Warning,
}

/// CSX Compiler, give me css and i will give you more css
#[wasm_bindgen(js_name = Compiler)]
pub struct JsCompiler {
    modules: HashMap<String, HashMap<String, Declaration>>,
}

#[wasm_bindgen(js_class = Compiler)]
impl JsCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new(options: Ts<JsCompilerOptions>) -> Result<JsCompiler, JsError> {
        options.to_rust()?;
        Ok(Self {
            modules: HashMap::new(),
        })
    }

    #[wasm_bindgen(js_name = compileModule)]
    pub fn compile_module(
        &mut self,
        options: Ts<JsCompileModuleOptions>,
    ) -> Result<Ts<JsCompileModuleResult>, JsError> {
        let options = options.to_rust()?;

        let blocks = parse::parse(&options.source)?;
        let mut result = JsCompileModuleResult::default();
        let mut atomic_declarations: HashMap<String, Declaration> = HashMap::new();
        let mut base_locations = HashSet::new();

        for ClassBlock {
            name,
            condition,
            in_media,
            location,
            declarations,
        } in &blocks
        {
            let position = ExportLocation {
                line: location.line,
                column: location.column - 1,
            };
            if !in_media && *condition == Condition::Base && base_locations.insert(name) {
                result.export_locations.insert(name.clone(), position);
            } else {
                result
                    .export_locations
                    .entry(name.clone())
                    .or_insert(position);
            }
            let exports = result.exports.entry(name.clone()).or_default();
            for declaration in declarations {
                let css = &declaration.css;
                let mut hasher = DefaultHasher::new();
                (&options.id, css).hash(&mut hasher);
                let mut key = if declaration.condition == Condition::Base {
                    declaration.property.clone()
                } else {
                    let suffix = declaration.condition.suffix();
                    suffix.hash(&mut hasher);
                    format!("{suffix}:{}", declaration.property)
                };
                if let Some(media) = &declaration.media {
                    // Sharing atoms across source rules can change which overlapping query wins.
                    (&media.query, media.position).hash(&mut hasher);
                    key = format!("@media {}:{}:{key}", media.query.len(), media.query);
                }
                let atomic_class = format!("c{:08x}x", hasher.finish() & 0xffff_ffff);

                let previous = atomic_declarations
                    .entry(atomic_class.clone())
                    .or_insert_with(|| declaration.clone());
                if *previous != *declaration {
                    return Err(JsError::new(&format!(
                        "Atomic class hash collision for {atomic_class}"
                    )));
                }
                exports.insert(key, atomic_class);
            }
        }

        let exported_atoms: HashSet<_> = result
            .exports
            .values()
            .flat_map(|style| style.values())
            .collect();
        atomic_declarations.retain(|name, _| exported_atoms.contains(name));
        let result = result.into_ts()?;
        self.modules.insert(options.id, atomic_declarations);
        Ok(result)
    }

    #[wasm_bindgen(js_name = removeModule)]
    pub fn remove_module(&mut self, id: &str) {
        self.modules.remove(id);
    }

    pub fn emit(&self, module_ids: Option<Vec<String>>) -> Result<String, JsError> {
        let modules: Vec<_> = match &module_ids {
            Some(ids) => ids
                .iter()
                .map(|id| {
                    self.modules
                        .get_key_value(id)
                        .ok_or_else(|| JsError::new(&format!("CSX module was not compiled: {id}")))
                })
                .collect::<Result<_, _>>()?,
            None => self.modules.iter().collect(),
        };
        let mut atoms = HashMap::new();
        for (id, module) in modules {
            for (name, declaration) in module {
                if let Some((previous_id, previous)) = atoms.insert(name, (id, declaration))
                    && (previous != declaration
                        || (declaration.media.is_some() && previous_id != id))
                {
                    return Err(JsError::new(&format!(
                        "Atomic class hash collision for {name}"
                    )));
                }
            }
        }
        if atoms.is_empty() {
            return Ok(String::new());
        }

        let mut atoms: Vec<_> = atoms.into_iter().collect();
        atoms.sort_unstable_by_key(|(name, (id, declaration))| {
            (
                declaration.layer,
                declaration.media.is_some(),
                declaration.condition,
                declaration
                    .media
                    .as_ref()
                    .map(|media| (*id, media.position)),
                Reverse(declaration.shorthand_width),
                declaration.property.as_str(),
                *name,
            )
        });

        let mut layers = Vec::new();
        for (layer, tier) in [
            (Layer::Reset, "reset"),
            (Layer::Shorthand, "shorthand"),
            (Layer::Longhand, "longhand"),
        ] {
            for (in_media, group) in [(false, "unconditional"), (true, "media")] {
                for (condition, state) in [
                    (Condition::Base, "base"),
                    (Condition::Hover, "hover"),
                    (Condition::Focus, "focus"),
                    (Condition::FocusVisible, "focus-visible"),
                ] {
                    layers.push((
                        layer,
                        in_media,
                        condition,
                        format!("csx.{tier}.{group}.{state}"),
                    ));
                }
            }
        }
        let names: Vec<_> = layers.iter().map(|(_, _, _, name)| name.as_str()).collect();
        let mut css = format!("@layer {};\n", names.join(", "));
        for (layer, in_media, condition, name) in layers {
            let mut rules = atoms
                .iter()
                .filter(|(_, (_, declaration))| {
                    declaration.layer == layer
                        && declaration.media.is_some() == in_media
                        && declaration.condition == condition
                })
                .peekable();
            if rules.peek().is_none() {
                continue;
            }
            css.push_str(&format!("\n@layer {name} {{\n"));
            let suffix = condition.suffix();
            let mut open_query = None;
            for (class, (_, declaration)) in rules {
                let query = declaration.media.as_ref().map(|media| media.query.as_str());
                if open_query != query {
                    if open_query.is_some() {
                        css.push_str("  }\n");
                    }
                    if let Some(query) = query {
                        css.push_str(&format!("  @media {query} {{\n"));
                    }
                    open_query = query;
                }
                let indent = if in_media { "    " } else { "  " };
                css.push_str(&format!(
                    "{indent}.{class}{suffix} {{ {}; }}\n",
                    declaration.css
                ));
            }
            if open_query.is_some() {
                css.push_str("  }\n");
            }
            css.push_str("}\n");
        }
        Ok(css)
    }
}
