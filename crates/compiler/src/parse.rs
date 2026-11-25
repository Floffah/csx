use lightningcss::declaration::DeclarationBlock;
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::{Property, PropertyId};
use lightningcss::rules::{CssRule, CssRuleList, Location};
use lightningcss::selector::PseudoClass::{Focus, FocusVisible, Hover};
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use lightningcss::visitor::{Visit, VisitTypes, Visitor};
use parcel_selectors::parser::Component::{Class, NonTSPseudoClass};
use wasm_bindgen::JsError;

pub(super) struct ClassBlock {
    pub(super) name: String,
    pub(super) condition: Condition,
    pub(super) in_media: bool,
    pub(super) location: Location,
    pub(super) declarations: Vec<Declaration>,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Media {
    pub(super) query: String,
    pub(super) position: (u32, u32),
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
    Focus,
    FocusVisible,
}

impl Condition {
    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Base => "",
            Self::Hover => ":hover",
            Self::Focus => ":focus",
            Self::FocusVisible => ":focus-visible",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Declaration {
    pub(super) property: String,
    pub(super) css: String,
    pub(super) layer: Layer,
    pub(super) condition: Condition,
    pub(super) media: Option<Media>,
    pub(super) shorthand_width: usize,
}

impl Declaration {
    fn new(
        property: &Property,
        name: String,
        condition: Condition,
        media: Option<&Media>,
    ) -> Result<Self, JsError> {
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
            media: media.cloned(),
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

pub(super) fn parse(source: &str) -> Result<Vec<ClassBlock>, JsError> {
    let mut sheet = StyleSheet::parse(source, ParserOptions::default())?;
    sheet.visit(&mut RejectImportant)?;
    let mut blocks = Vec::new();
    parse_rules(sheet.rules, None, &mut blocks)?;
    Ok(blocks)
}

fn parse_rules(
    rules: CssRuleList<'_>,
    query: Option<&str>,
    blocks: &mut Vec<ClassBlock>,
) -> Result<(), JsError> {
    for rule in rules.0 {
        match rule {
            CssRule::Media(media) if query.is_none() => {
                let query = media.query.to_css_string(PrinterOptions::default())?;
                parse_rules(media.rules, Some(&query), blocks)?;
            }
            CssRule::Style(style) => {
                let line = style.loc.line + 1;
                if !style.rules.0.is_empty() {
                    return Err(JsError::new(&format!(
                        "Nested rules are not supported in CSX stylesheets (line {line})"
                    )));
                }
                let media = query.map(|query| Media {
                    query: query.to_owned(),
                    position: (style.loc.line, style.loc.column),
                });
                for selector in &style.selectors.0 {
                    let (class, condition) = match selector.iter_raw_match_order().as_slice() {
                        [Class(class)] => (class, Condition::Base),
                        [Class(class), NonTSPseudoClass(Hover)]
                        | [NonTSPseudoClass(Hover), Class(class)] => (class, Condition::Hover),
                        [Class(class), NonTSPseudoClass(Focus)]
                        | [NonTSPseudoClass(Focus), Class(class)] => (class, Condition::Focus),
                        [Class(class), NonTSPseudoClass(FocusVisible)]
                        | [NonTSPseudoClass(FocusVisible), Class(class)] => {
                            (class, Condition::FocusVisible)
                        }
                        _ => {
                            let selector = selector.to_css_string(PrinterOptions::default())?;
                            return Err(JsError::new(&format!(
                                "Unsupported selector `{selector}` in CSX stylesheet (line {line}). Use a single class with an optional :hover, :focus, or :focus-visible."
                            )));
                        }
                    };

                    let mut declarations = Vec::new();
                    for property in &style.declarations.declarations {
                        let name = property
                            .property_id()
                            .to_css_string(PrinterOptions::default())?;
                        declarations.push(Declaration::new(
                            property,
                            name,
                            condition,
                            media.as_ref(),
                        )?);
                    }
                    blocks.push(ClassBlock {
                        name: class.0.to_string(),
                        condition,
                        in_media: media.is_some(),
                        location: style.loc,
                        declarations,
                    });
                }
            }
            rule => {
                let css = rule.to_css_string(PrinterOptions::default())?;
                let name = css
                    .split(|c: char| c.is_whitespace() || c == '{' || c == ';')
                    .next()
                    .filter(|name| !name.is_empty())
                    .unwrap_or("rule");
                return Err(JsError::new(&format!(
                    "Unsupported rule `{name}` in CSX stylesheet"
                )));
            }
        }
    }

    Ok(())
}
