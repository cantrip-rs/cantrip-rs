use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// Mowu, the legendary 3/3 green Dog token Jiang Yanggu makes (CR 111.1, 205.4a).
pub static MOWU: CardDef = CardDef {
    name: "Mowu",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    subtypes: SubtypeSet::of(&[Subtype::Dog]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// "When Jiang Yanggu enters, create Mowu, a legendary 3/3 green Dog creature token."
fn make_mowu(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &MOWU, 1)?;
    Ok(())
}

/// "At the beginning of your end step, untap all tokens you control." Every token under this permanent's controller's
/// control as the ability resolves (CR 608.2c), tapped or not; a noncreature token untaps too (CR 111.1).
fn untap_tokens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tokens: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.obj(id).token).collect();
    g.untap_together(&tokens);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, make_mowu, "When Jiang Yanggu enters, create Mowu, a legendary 3/3 green Dog creature token."),
        triggered(triggers::YOUR_END_STEP, untap_tokens, "At the beginning of your end step, untap all tokens you control."),
    ],
    ..oracle::JIANG_YANGGU_NEVER_ALONE
};

pub static EXTRA: &[&CardDef] = &[&MOWU];
