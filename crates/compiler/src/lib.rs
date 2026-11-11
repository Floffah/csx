//! Rust compiler for CSX. Implementation has not started.

use lightningcss::declaration::DeclarationBlock;
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::Property;
use lightningcss::properties::custom::{CustomPropertyName, Token, TokenOrValue};
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use lightningcss::values::syntax::ParsedComponent;
use parcel_selectors::parser::Component;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
                        let css = declaration.to_css(declaration.important);
                        let mut hasher = DefaultHasher::new();
                        (&options.id, &css).hash(&mut hasher);
                        let atomic_class = format!("c{:07x}", hasher.finish() & 0x0fff_ffff);

                        let previous = atomic_declarations
                            .entry(atomic_class.clone())
                            .or_insert_with(|| (css.clone(), declaration.important));
                        if previous.0 != css {
                            return Err(JsError::new(&format!(
                                "Atomic class hash collision for {atomic_class}"
                            )));
                        }
                        if declaration.important
                            || !exports
                                .get(&declaration.property)
                                .is_some_and(|name| atomic_declarations[name].1)
                        {
                            exports.insert(declaration.property.clone(), atomic_class);
                        }
                    }
                }
                CollectedBlock::Tokens { name, declarations } => {
                    let variables = result.variables.entry(name.clone()).or_default();
                    for declaration in declarations {
                        let mut hasher = DefaultHasher::new();
                        declaration.to_css(false).hash(&mut hasher);
                        variables.insert(
                            declaration.property.clone(),
                            format!("--c{:07x}", hasher.finish() & 0x0fff_ffff),
                        );
                    }
                }
            }
        }

        let result = result.into_ts()?;
        self.modules.insert(options.id, declarations);
        Ok(result)
    }
}

#[derive(Default)]
struct ModuleDeclarations {
    blocks: Vec<CollectedBlock>,
    /// Generated class name -> (serialized declaration, importance).
    atomic_declarations: HashMap<String, (String, bool)>,
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

struct Declaration {
    property: String,
    value: String,
    important: bool,
    // Keep the printer's property spelling and spacing for the existing hashes.
    css_prefix: String,
}

impl Declaration {
    fn new(property: &Property, name: String, important: bool) -> Result<Self, JsError> {
        let value = property.value_to_css_string(PrinterOptions::default())?;
        let css = property.to_css_string(false, PrinterOptions::default())?;
        let css_prefix = css
            .strip_suffix(&value)
            .ok_or_else(|| JsError::new("Could not separate the declaration value"))?
            .to_owned();
        Ok(Self {
            property: name,
            value,
            important,
            css_prefix,
        })
    }

    fn to_css(&self, important: bool) -> String {
        let suffix = if important { " !important" } else { "" };
        format!("{}{}{suffix}", self.css_prefix, self.value)
    }
}

impl ModuleDeclarations {
    fn parse(source: &str) -> Result<Self, JsError> {
        let sheet = StyleSheet::parse(source, ParserOptions::default())?;
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
                        for (property, important) in style.declarations.iter() {
                            let name = property
                                .property_id()
                                .to_css_string(PrinterOptions::default())?;
                            declarations.push(Declaration::new(property, name, important)?);
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
                    let block = DeclarationBlock::parse_string(
                        &css,
                        ParserOptions {
                            error_recovery: true,
                            ..ParserOptions::default()
                        },
                    )
                    .map_err(|error| JsError::new(&error.to_string()))?;

                    let mut declarations = Vec::new();
                    for (property, important) in block.iter() {
                        let Property::Custom(custom) = property else {
                            continue;
                        };
                        let CustomPropertyName::Custom(name) = &custom.name else {
                            continue;
                        };
                        declarations.push(Declaration::new(
                            property,
                            name.0.to_string(),
                            important,
                        )?);
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
