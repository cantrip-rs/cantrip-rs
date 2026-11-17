use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{copy, library};
use cantrip_oracle as oracle;

/// "Exile a card from your hand": any card of it (CR 602.1a, 118.3).
fn any_card(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
    true
}

/// "Activate only ... if this card is exiled" (CR 602.5b): the ability is the exiled card's own only while it is there, so a
/// card that left exile and came back is a new object (CR 400.7) the grant no longer reaches, and one that went anywhere else
/// can't be activated where it is (CR 113.6b).
fn still_exiled(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).zone.is(ZoneKind::Exile)
}

/// "Create a copy of this card. You may cast the copy." (CR 707.12, 608.2g): the copy is made in exile from the card as it
/// is, and cast as the ability resolves for its own costs, or not at all - a copy that isn't cast ceases to exist (CR 707.10a,
/// 704.5e). A copy of a permanent spell becomes a token (CR 707.10f, 608.3f).
fn copy_and_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    copy::may_cast_a_copy_paying_costs(g, item.controller, item.object)?;
    Ok(())
}

/// "When Q enters, shuffle your library, then exile cards from the top of your library until you exile a nonland card. That
/// nonland card gains ..." (CR 701.13a, 613.1f): the shuffle happens whether or not a nonland card is found (the card has no
/// "if you do"), and the ability the card gains is this card's delayed ability 0, for as long as the game lasts.
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.shuffle(item.controller);
    g.end_sentence();
    let Some(card) = library::exile_until_any_nonland(g, item.controller, None) else { return Ok(()) };
    g.end_sentence();
    g.add_effect(item.object, card, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        enters,
        "When Q enters, shuffle your library, then exile cards from the top of your library until you exile a nonland card. That nonland card gains \"Exile a card from your hand: Create a copy of this card. You may cast the copy. Activate only during your turn and only if this card is exiled.\"",
    )],
    delayed: &[ability(
        Cost { exile_from_hand: Some((1, any_card)), ..Cost::NONE },
        copy_and_cast,
        "Exile a card from your hand: Create a copy of this card. You may cast the copy. Activate only during your turn and only if this card is exiled.",
    )
    .from(ZoneKind::Exile)
    .timing(Timing::OwnTurn)
    .condition(still_exiled)],
    ..oracle::Q_STACKING_THE_DECK
};
