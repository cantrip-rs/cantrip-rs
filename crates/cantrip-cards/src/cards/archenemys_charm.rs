use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature and/or planeswalker card in your graveyard" (CR 115.1).
fn creature_or_planeswalker_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature, CardType::Planeswalker])
}

/// "Put two +1/+1 counters on target creature you control. It gains lifelink until end of turn."
fn counters_and_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 2);
    effects::gain_lifelink_until_end_of_turn(g, item.object, creature);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::exile_first_target, "Exile target creature or planeswalker.").targets(&[targets::CREATURE_OR_PLANESWALKER]),
    mode(effects::return_each_target_to_hand, "Return one or two target creature and/or planeswalker cards from your graveyard to your hand.")
        .targets(&[target(creature_or_planeswalker_card).between(1, 2)]),
    mode(counters_and_lifelink, "Put two +1/+1 counters on target creature you control. It gains lifelink until end of turn.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::ARCHENEMYS_CHARM };
