use cantrip_api::*;
use cantrip_mechanics::effects::return_first_target_to_battlefield;
use cantrip_mechanics::{choose, select, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "You may tap another untapped Merfolk you control. When you do, ..." (CR 603.12, 701.26): the Merfolk is chosen as the ability
/// resolves, an untapped one other than this creature, read as this card's text says Merfolk (CR 612); the reflexive ability
/// triggers only if one was tapped, and chooses its target as it goes on the stack.
fn tap_a_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let merfolk = g.words(item.object).subtype(Subtype::Merfolk);
    let untapped: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != item.object && g.controller(id) == you && !g.state().obj(id).tapped && g.chars(id).has_subtype(merfolk))
        .collect();
    let Some(&chosen) = choose::select(g, you, item.object, Purpose::Tap, untapped.clone(), Selection::listed(select::up_to(1), &untapped, untapped.clone()))?.first() else {
        return Ok(());
    };
    g.tap(chosen);
    let this = g.card_id(&CARD);
    g.reflexive_trigger(item.object, you, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        tap_a_merfolk,
        "Whenever this creature attacks, you may tap another untapped Merfolk you control. When you do, return target creature card with mana value 3 or less from your graveyard to the battlefield.",
    )],
    delayed: &[delayed_trigger(
        return_first_target_to_battlefield,
        "When you do, return target creature card with mana value 3 or less from your graveyard to the battlefield.",
    )
    .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::MEANDERS_GUIDE
};
