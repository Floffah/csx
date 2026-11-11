//! Rust compiler for CSX.

mod parse;

use parse::{CollectedBlock, Condition, Declaration, Layer};
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
    /// Conditional keys include the selector suffix, for example `:hover:color`.
    pub exports: HashMap<String, HashMap<String, String>>,
    /// Token block name -> token property name -> generated CSS variable name.
    pub variables: HashMap<String, HashMap<String, String>>,
    /// Canonical IDs of modules this compilation depends on.
    pub dependencies: Vec<String>,
    #[tsify(type = "Diagnostic[]")]
    pub diagnostics: Vec<JsDiagnostic>,
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
    options: JsCompilerOptions,
    modules: HashMap<String, HashMap<String, Declaration>>,
}

#[wasm_bindgen(js_class = Compiler)]
impl JsCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new(options: Ts<JsCompilerOptions>) -> Result<JsCompiler, JsError> {
        let options = options.to_rust()?;
        return Ok(Self {
            options,
            modules: HashMap::new(),
        });
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

        for block in &blocks {
            match block {
                CollectedBlock::Class { name, declarations } => {
                    let exports = result.exports.entry(name.clone()).or_default();
                    for declaration in declarations {
                        let css = &declaration.css;
                        let mut hasher = DefaultHasher::new();
                        (&options.id, css).hash(&mut hasher);
                        let key = if declaration.condition == Condition::Base {
                            declaration.property.clone()
                        } else {
                            let suffix = declaration.condition.suffix();
                            suffix.hash(&mut hasher);
                            format!("{suffix}:{}", declaration.property)
                        };
                        let atomic_class = format!("c{:07x}", hasher.finish() & 0x0fff_ffff);

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
                CollectedBlock::Tokens { name, declarations } => {
                    let variables = result.variables.entry(name.clone()).or_default();
                    for declaration in declarations {
                        let mut hasher = DefaultHasher::new();
                        declaration.css.hash(&mut hasher);
                        variables.insert(
                            declaration.property.clone(),
                            format!("--c{:07x}", hasher.finish() & 0x0fff_ffff),
                        );
                    }
                }
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

    pub fn emit(&self, module_ids: Option<Vec<String>>) -> Result<String, JsError> {
        let modules: Vec<_> = match &module_ids {
            Some(ids) => ids
                .iter()
                .map(|id| {
                    self.modules
                        .get(id)
                        .ok_or_else(|| JsError::new(&format!("CSX module was not compiled: {id}")))
                })
                .collect::<Result<_, _>>()?,
            None => self.modules.values().collect(),
        };
        let mut atoms = HashMap::new();
        for module in modules {
            for (name, declaration) in module {
                if let Some(previous) = atoms.insert(name, declaration)
                    && previous != declaration
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
        atoms.sort_unstable_by_key(|(name, declaration)| {
            (
                declaration.layer,
                declaration.condition,
                Reverse(declaration.shorthand_width),
                declaration.property.as_str(),
                *name,
            )
        });

        let layers = [
            (
                Layer::Reset,
                Condition::Base,
                "csx.reset.unconditional.base",
            ),
            (
                Layer::Reset,
                Condition::Hover,
                "csx.reset.unconditional.hover",
            ),
            (
                Layer::Shorthand,
                Condition::Base,
                "csx.shorthand.unconditional.base",
            ),
            (
                Layer::Shorthand,
                Condition::Hover,
                "csx.shorthand.unconditional.hover",
            ),
            (
                Layer::Longhand,
                Condition::Base,
                "csx.longhand.unconditional.base",
            ),
            (
                Layer::Longhand,
                Condition::Hover,
                "csx.longhand.unconditional.hover",
            ),
        ];
        let mut css = format!("@layer {};\n", layers.map(|(_, _, name)| name).join(", "));
        for (layer, condition, name) in layers {
            let mut rules = atoms
                .iter()
                .filter(|(_, declaration)| {
                    declaration.layer == layer && declaration.condition == condition
                })
                .peekable();
            if rules.peek().is_none() {
                continue;
            }
            css.push_str(&format!("\n@layer {name} {{\n"));
            let suffix = condition.suffix();
            for (class, declaration) in rules {
                css.push_str(&format!("  .{class}{suffix} {{ {}; }}\n", declaration.css));
            }
            css.push_str("}\n");
        }
        Ok(css)
    }
}
