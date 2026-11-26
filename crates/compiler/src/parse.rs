use lightningcss::declaration::DeclarationBlock;
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::{Property, PropertyId};
use lightningcss::rules::{CssRule, CssRuleList, Location};
use lightningcss::selector::PseudoClass::{
    self, Active, Disabled, Focus, FocusVisible, FocusWithin, Hover, Invalid, Required,
};
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

// Later entries take precedence within each property tier and media group.
type SelectorState<'i> = (Option<PseudoClass<'i>>, &'static str, &'static str);

fn selector_states<'i>() -> &'i [SelectorState<'i>] {
    const {
        &[
            (None, "", "base"),
            (Some(Hover), ":hover", "hover"),
            (Some(Focus), ":focus", "focus"),
            (Some(FocusVisible), ":focus-visible", "focus-visible"),
            (Some(Active), ":active", "active"),
            (Some(Disabled), ":disabled", "disabled"),
            (Some(FocusWithin), ":focus-within", "focus-within"),
            (Some(Required), ":required", "required"),
            (Some(Invalid), ":invalid", "invalid"),
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Condition(usize);

impl Condition {
    pub(super) fn ordered() -> impl Iterator<Item = (Self, &'static str)> {
        selector_states()
            .iter()
            .enumerate()
            .map(|(index, (_, _, layer))| (Self(index), *layer))
    }

    pub(super) fn suffix(self) -> &'static str {
        selector_states()[self.0].1
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
                    let state = match selector.iter_raw_match_order().as_slice() {
                        [Class(class)] => Some((class, None)),
                        [Class(class), NonTSPseudoClass(pseudo)]
                        | [NonTSPseudoClass(pseudo), Class(class)] => Some((class, Some(pseudo))),
                        _ => None,
                    };
                    let Some((class, condition)) = state.and_then(|(class, pseudo)| {
                        selector_states()
                            .iter()
                            .position(|(known, _, _)| known.as_ref() == pseudo)
                            .map(|index| (class, Condition(index)))
                    }) else {
                        let selector = selector.to_css_string(PrinterOptions::default())?;
                        let supported = selector_states()
                            .iter()
                            .filter_map(|(pseudo, suffix, _)| pseudo.as_ref().map(|_| *suffix))
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(JsError::new(&format!(
                            "Unsupported selector `{selector}` in CSX stylesheet (line {line}). Use a single class with an optional pseudo-class ({supported})."
                        )));
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
