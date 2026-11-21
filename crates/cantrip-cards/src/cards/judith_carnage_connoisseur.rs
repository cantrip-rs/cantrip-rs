use cantrip_api::*;
use cantrip_mechanics::triggers::{INSTANT_OR_SORCERY_YOU_CAST, THIS_DIES};
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// The 2/2 red Imp creature token with "When this token dies, it deals 2 damage to each opponent."
pub static IMP: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[triggered(THIS_DIES, effects::damage_each_opponent_n::<2>, "When this token dies, it deals 2 damage to each opponent.")],
    text: "When this token dies, it deals 2 damage to each opponent.",
    ..token!(Imp)
};

/// "That spell gains deathtouch and lifelink" (CR 702.2, 702.15, 613.1f): the spell that was cast, which the trigger is about
/// (`Fired::about`), has both until it leaves the stack. The two are one instruction (CR 608.2c). A spell already gone (the
/// trigger resolves first, so it is gone only if something removed it) gains nothing (CR 400.7).
fn gain_deathtouch_and_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    g.add_effect(item.object, spell, EffectKind::AddKeywords(Keywords::of(&[Keyword::Deathtouch, Keyword::Lifelink])), Duration::Indefinite);
    Ok(())
}

/// "Create a 2/2 red Imp creature token with ..."
fn create_imp(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &IMP, 1)?;
    Ok(())
}

/// "Whenever you cast an instant or sorcery spell, choose one —" (CR 603.2, 700.2).
static MODAL: ModalDef = modes(&[
    mode(gain_deathtouch_and_lifelink, "That spell gains deathtouch and lifelink."),
    mode(create_imp, "Create a 2/2 red Imp creature token with \"When this token dies, it deals 2 damage to each opponent.\""),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        INSTANT_OR_SORCERY_YOU_CAST,
        &MODAL,
        "Whenever you cast an instant or sorcery spell, choose one — that spell gains deathtouch and lifelink; or create a 2/2 red Imp creature token with \"When this token dies, it deals 2 damage to each opponent.\"",
    )],
    ..oracle::JUDITH_CARNAGE_CONNOISSEUR
};

pub static EXTRA: &[&CardDef] = &[&IMP];
