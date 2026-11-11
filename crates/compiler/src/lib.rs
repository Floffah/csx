//! Rust compiler for CSX.

use lightningcss::declaration::DeclarationBlock;
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::custom::{CustomPropertyName, Token, TokenOrValue};
use lightningcss::properties::{Property, PropertyId};
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use lightningcss::values::syntax::ParsedComponent;
use lightningcss::visitor::{Visit, VisitTypes, Visitor};
use parcel_selectors::parser::Component;
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
    /// User class name -> property name -> generated atomic class name.
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
    modules: HashMap<String, ModuleDeclarations>,
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

        let mut declarations = ModuleDeclarations::parse(&options.source)?;
        let mut result = JsCompileModuleResult::default();
        let atomic_declarations = &mut declarations.atomic_declarations;

        for block in &declarations.blocks {
            match block {
                CollectedBlock::Class { name, declarations } => {
                    let exports = result.exports.entry(name.clone()).or_default();
                    for declaration in declarations {
                        let css = &declaration.css;
                        let mut hasher = DefaultHasher::new();
                        (&options.id, css).hash(&mut hasher);
                        let atomic_class = format!("c{:07x}", hasher.finish() & 0x0fff_ffff);

                        let previous = atomic_declarations
                            .entry(atomic_class.clone())
                            .or_insert_with(|| declaration.clone());
                        if &previous.css != css {
                            return Err(JsError::new(&format!(
                                "Atomic class hash collision for {atomic_class}"
                            )));
                        }
                        exports.insert(declaration.property.clone(), atomic_class);
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
        self.modules.insert(options.id, declarations);
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
            for (name, declaration) in &module.atomic_declarations {
                if let Some(previous) = atoms.insert(name, declaration)
                    && previous.css != declaration.css
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
                Reverse(declaration.shorthand_width),
                declaration.property.as_str(),
                *name,
            )
        });

        let layers = [
            (Layer::Reset, "csx.reset.unconditional.base"),
            (Layer::Shorthand, "csx.shorthand.unconditional.base"),
            (Layer::Longhand, "csx.longhand.unconditional.base"),
        ];
        let mut css = format!("@layer {};\n", layers.map(|(_, name)| name).join(", "));
        for (layer, name) in layers {
            let mut rules = atoms
                .iter()
                .filter(|(_, declaration)| declaration.layer == layer)
                .peekable();
            if rules.peek().is_none() {
                continue;
            }
            css.push_str(&format!("\n@layer {name} {{\n"));
            for (class, declaration) in rules {
                css.push_str(&format!("  .{class} {{ {}; }}\n", declaration.css));
            }
            css.push_str("}\n");
        }
        Ok(css)
    }
}

#[derive(Default)]
struct ModuleDeclarations {
    blocks: Vec<CollectedBlock>,
    atomic_declarations: HashMap<String, Declaration>,
}

enum CollectedBlock {
    Class {
        name: String,
        declarations: Vec<Declaration>,
    },
    Tokens {
        name: String,
        declarations: Vec<Declaration>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Layer {
    Reset,
    Shorthand,
    Longhand,
}

#[derive(Clone)]
struct Declaration {
    property: String,
    css: String,
    layer: Layer,
    shorthand_width: usize,
}

impl Declaration {
    fn new(property: &Property, name: String) -> Result<Self, JsError> {
        let id = property.property_id();
        let (layer, shorthand_width) = match id {
            PropertyId::All => (Layer::Reset, 0),
            _ => match id.longhands() {
                Some(longhands) => (Layer::Shorthand, longhands.len()),
                None => (Layer::Longhand, 0),
            },
        };
        Ok(Self {
            property: name,
            css: property.to_css_string(false, PrinterOptions::default())?,
            layer,
            shorthand_width,
        })
    }
}

struct RejectImportant;

impl<'i> Visitor<'i> for RejectImportant {
    type Error = JsError;

    fn visit_types(&self) -> VisitTypes {
        VisitTypes::PROPERTIES
    }

    fn visit_declaration_block(&mut self, block: &mut DeclarationBlock<'i>) -> Result<(), JsError> {
        if !block.important_declarations.is_empty() {
            return Err(JsError::new(
                "!important is not supported in CSX stylesheets",
            ));
        }
        Ok(())
    }
}

impl ModuleDeclarations {
    fn parse(source: &str) -> Result<Self, JsError> {
        let mut sheet = StyleSheet::parse(source, ParserOptions::default())?;
        sheet.visit(&mut RejectImportant)?;
        let mut module = Self::default();

        for rule in sheet.rules.0 {
            match rule {
                CssRule::Style(style) => {
                    for selector in &style.selectors.0 {
                        let [Component::Class(class)] = selector.iter_raw_match_order().as_slice()
                        else {
                            continue;
                        };

                        let mut declarations = Vec::new();
                        for property in &style.declarations.declarations {
                            let name = property
                                .property_id()
                                .to_css_string(PrinterOptions::default())?;
                            declarations.push(Declaration::new(property, name)?);
                        }
                        module.blocks.push(CollectedBlock::Class {
                            name: class.0.to_string(),
                            declarations,
                        });
                    }
                }
                CssRule::Unknown(at_rule) if at_rule.name == "tokens" => {
                    let Some(TokenOrValue::Token(Token::Ident(name))) = at_rule.prelude.0.first()
                    else {
                        continue;
                    };
                    let Some(block) = at_rule.block else {
                        continue;
                    };
                    let css = ParsedComponent::TokenList(block)
                        .to_css_string(PrinterOptions::default())?;
                    let mut block = DeclarationBlock::parse_string(
                        &css,
                        ParserOptions {
                            error_recovery: true,
                            ..ParserOptions::default()
                        },
                    )
                    .map_err(|error| JsError::new(&error.to_string()))?;
                    block.visit(&mut RejectImportant)?;

                    let mut declarations = Vec::new();
                    for property in &block.declarations {
                        let Property::Custom(custom) = property else {
                            continue;
                        };
                        let CustomPropertyName::Custom(name) = &custom.name else {
                            continue;
                        };
                        declarations.push(Declaration::new(property, name.0.to_string())?);
                    }
                    module.blocks.push(CollectedBlock::Tokens {
                        name: name.to_string(),
                        declarations,
                    });
                }
                _ => (),
            }
        }

        Ok(module)
    }
}
