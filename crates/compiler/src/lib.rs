//! Rust compiler for CSX. Implementation has not started.

use lightningcss::printer::PrinterOptions;
use lightningcss::properties::custom::{Token, TokenList, TokenOrValue};
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use lightningcss::values::ident::DashedIdent;
use lightningcss::values::syntax::ParsedComponent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
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

#[derive(Default, Debug)]
struct TokensBlock {
    name: String,
    block: HashMap<String, String>,
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

        // log(format!("{sheet:#?}").as_str());

        let mut token_blocks: Vec<TokensBlock> = vec![];

        for rule in sheet.rules.0 {
            match rule {
                CssRule::Unknown(at_rule) => {
                    let name = at_rule.name;
                    if (name != "tokens" || at_rule.prelude.0.len() <= 0) {
                        continue;
                    }

                    let tokens_name: Option<String> = match at_rule.prelude.0.first() {
                        Some(TokenOrValue::Token(Token::Ident(name))) => Some(name.to_string()),
                        _ => continue,
                    };

                    // log(tokens_name.unwrap().as_str())

                    let parsed_tokens: Option<Vec<TokenOrValue>> = match at_rule.block {
                        Some(TokenList(array)) => Some(array),
                        None => None,
                    };
                    if parsed_tokens.is_none() {
                        continue;
                    }

                    let mut tokens_block = TokensBlock {
                        name: tokens_name.unwrap(),
                        block: HashMap::new(),
                    };
                    let parsed_tokens = parsed_tokens.unwrap();

                    let mut current_token_ident = String::default();
                    let mut token_body_accum: Vec<TokenOrValue> = vec![];

                    for token in parsed_tokens {
                        match token {
                            TokenOrValue::DashedIdent(DashedIdent(ident)) => {
                                current_token_ident = ident.to_string()
                            }
                            TokenOrValue::Token(Token::WhiteSpace(_)) => {}
                            TokenOrValue::Token(Token::Semicolon) => {
                                let value = ParsedComponent::TokenList(TokenList(std::mem::take(
                                    &mut token_body_accum,
                                )))
                                .to_css_string(PrinterOptions::default())?;
                                let value = value.trim().strip_prefix(":");

                                tokens_block
                                    .block
                                    .insert(current_token_ident.clone(), value.unwrap_or("").to_string());

                                current_token_ident = String::default();
                                token_body_accum = vec![];
                            }
                            token => token_body_accum.push(token),
                        }
                    }

                    token_blocks.push(tokens_block);
                }
                _ => (),
            }

            log(format!("{token_blocks:#?}").as_str());
        }

        Ok(JsCompileModuleResult::default().into_ts()?)
    }
}
