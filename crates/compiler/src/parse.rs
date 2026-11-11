use lightningcss::declaration::DeclarationBlock;
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::custom::{CustomPropertyName, Token, TokenOrValue};
use lightningcss::properties::{Property, PropertyId};
use lightningcss::rules::{CssRule, Location};
use lightningcss::selector::PseudoClass;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use lightningcss::values::syntax::ParsedComponent;
use lightningcss::visitor::{Visit, VisitTypes, Visitor};
use parcel_selectors::parser::Component;
use wasm_bindgen::JsError;

pub(super) enum CollectedBlock {
    Class {
        name: String,
        condition: Condition,
        location: Location,
        declarations: Vec<Declaration>,
    },
    Tokens {
        name: String,
        declarations: Vec<Declaration>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Layer {
    Reset,
    Shorthand,
    Longhand,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Condition {
    Base,
    Hover,
}

impl Condition {
    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Base => "",
            Self::Hover => ":hover",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Declaration {
    pub(super) property: String,
    pub(super) css: String,
    pub(super) layer: Layer,
    pub(super) condition: Condition,
    pub(super) shorthand_width: usize,
}

impl Declaration {
    fn new(property: &Property, name: String, condition: Condition) -> Result<Self, JsError> {
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
            condition,
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

pub(super) fn parse(source: &str) -> Result<Vec<CollectedBlock>, JsError> {
    let mut sheet = StyleSheet::parse(source, ParserOptions::default())?;
    sheet.visit(&mut RejectImportant)?;
    let mut blocks = Vec::new();

    for rule in sheet.rules.0 {
        match rule {
            CssRule::Style(style) => {
                for selector in &style.selectors.0 {
                    let (class, condition) = match selector.iter_raw_match_order().as_slice() {
                        [Component::Class(class)] => (class, Condition::Base),
                        [
                            Component::Class(class),
                            Component::NonTSPseudoClass(PseudoClass::Hover),
                        ]
                        | [
                            Component::NonTSPseudoClass(PseudoClass::Hover),
                            Component::Class(class),
                        ] => (class, Condition::Hover),
                        _ => continue,
                    };

                    let mut declarations = Vec::new();
                    for property in &style.declarations.declarations {
                        let name = property
                            .property_id()
                            .to_css_string(PrinterOptions::default())?;
                        declarations.push(Declaration::new(property, name, condition)?);
                    }
                    blocks.push(CollectedBlock::Class {
                        name: class.0.to_string(),
                        condition,
                        location: style.loc,
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
                let css =
                    ParsedComponent::TokenList(block).to_css_string(PrinterOptions::default())?;
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
                    declarations.push(Declaration::new(
                        property,
                        name.0.to_string(),
                        Condition::Base,
                    )?);
                }
                blocks.push(CollectedBlock::Tokens {
                    name: name.to_string(),
                    declarations,
                });
            }
            _ => (),
        }
    }

    Ok(blocks)
}
