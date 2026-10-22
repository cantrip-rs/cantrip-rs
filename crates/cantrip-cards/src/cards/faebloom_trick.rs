use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// The 1/1 blue Faerie creature token with flying (colors and creature type are those of the text that makes it, CR 612.4).
pub static FAERIE: CardDef = CardDef {
    name: "Faerie Token",
    token_name: Some(&[Subtype::Faerie]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Faerie]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..CardDef::BLANK
};

/// "Create two 1/1 blue Faerie creature tokens with flying. When you do, tap target creature an opponent controls."
/// The reflexive ability (CR 603.12) triggers only if the tokens were created, and its target is chosen as it is put on
/// the stack, so it is not chosen when the spell is cast (ruling); with no creature an opponent controls it is not put on
/// the stack.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let created = tokens::create(g, item.controller, &FAERIE, 2)?;
    if !created.is_empty() {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "Tap target creature an opponent controls" (CR 701.26a): a target that has become illegal is left alone (CR 608.2b).
fn tap_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.tap(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    delayed: &[delayed_trigger(tap_it, "When you do, tap target creature an opponent controls.").targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::FAEBLOOM_TRICK
};

pub static EXTRA: &[&CardDef] = &[&FAERIE];
