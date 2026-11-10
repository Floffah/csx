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
}

#[wasm_bindgen(js_class = Compiler)]
impl JsCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new(options: Ts<JsCompilerOptions>) -> Result<JsCompiler, JsError> {
        let options = options.to_rust()?;
        return Ok(Self { options });
    }

    #[wasm_bindgen(js_name = compileModule)]
    pub fn compile_module(
        &self,
        options: Ts<JsCompileModuleOptions>,
    ) -> Result<Ts<JsCompileModuleResult>, JsError> {
        let options = options.to_rust()?;

        let sheet = StyleSheet::parse(options.source.as_str(), ParserOptions::default())?;

        let mut result = JsCompileModuleResult::default();
        let mut declarations = HashMap::new();

        for rule in sheet.rules.0 {
            match rule {
                CssRule::Style(style) => {
                    for selector in &style.selectors.0 {
                        let [Component::Class(class)] = selector.iter_raw_match_order().as_slice()
                        else {
                            continue;
                        };
                        let exports = result.exports.entry(class.0.to_string()).or_default();

                        for (property, important) in style.declarations.iter() {
                            let name = property
                                .property_id()
                                .to_css_string(PrinterOptions::default())?;
                            let declaration =
                                property.to_css_string(important, PrinterOptions::default())?;
                            let mut hasher = DefaultHasher::new();
                            (&options.id, &declaration).hash(&mut hasher);
                            let atomic_class = format!("c{:07x}", hasher.finish() & 0x0fff_ffff);

                            let previous = declarations
                                .entry(atomic_class.clone())
                                .or_insert_with(|| (declaration.clone(), important));
                            if previous.0 != declaration {
                                return Err(JsError::new(&format!(
                                    "Atomic class hash collision for {atomic_class}"
                                )));
                            }
                            if important
                                || !exports.get(&name).is_some_and(|name| declarations[name].1)
                            {
                                exports.insert(name, atomic_class);
                            }
                        }
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
                    let variables = result.variables.entry(name.to_string()).or_default();
                    let css = ParsedComponent::TokenList(block)
                        .to_css_string(PrinterOptions::default())?;
                    let declarations = DeclarationBlock::parse_string(
                        &css,
                        ParserOptions {
                            error_recovery: true,
                            ..ParserOptions::default()
                        },
                    )
                    .map_err(|error| JsError::new(&error.to_string()))?;

                    for (property, _) in declarations.iter() {
                        let Property::Custom(custom) = property else {
                            continue;
                        };
                        let CustomPropertyName::Custom(name) = &custom.name else {
                            continue;
                        };
                        let declaration =
                            property.to_css_string(false, PrinterOptions::default())?;
                        let mut hasher = DefaultHasher::new();
                        declaration.hash(&mut hasher);
                        variables.insert(
                            name.0.to_string(),
                            format!("--c{:07x}", hasher.finish() & 0x0fff_ffff),
                        );
                    }
                }
                _ => (),
            }
        }

        Ok(result.into_ts()?)
    }
}
