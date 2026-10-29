use cantrip_api::*;
use cantrip_mechanics::prepare::become_prepared;
use cantrip_mechanics::triggers::CARDS_LEAVE_YOUR_GRAVEYARD;
use cantrip_mechanics::{targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// Kirol's prepare spell (CR 722): not a card here, only ever a copy. "Mill a card. Put two +1/+1 counters on target creature. It
/// gains trample until end of turn." The mill happens first, as the spell resolves (CR 608.2c); with the one target illegal the
/// spell does nothing (CR 608.2b), so no card is milled.
pub static PACK_A_PUNCH: CardDef = CardDef {
    name: "Pack a Punch",
    cost: mana("{1}{R}{W}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::CREATURE], mill_and_empower),
    text: "Mill a card. Put two +1/+1 counters on target creature. It gains trample until end of turn.",
    ..CardDef::BLANK
};

fn mill_and_empower(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.mill(item.controller, 1);
    g.add_counters(creature, CounterKind::PlusOne, 2);
    trample_until_end_of_turn(g, item.object, creature);
    Ok(())
}

/// "Whenever one or more cards leave your graveyard, Kirol becomes prepared": one trigger for the cards that left together (CR
/// 603.2c); nothing if it has left the battlefield by the time the ability resolves (CR 400.7).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        CARDS_LEAVE_YOUR_GRAVEYARD,
        become_prepared,
        "Whenever one or more cards leave your graveyard, Kirol becomes prepared.",
    )
    .once_per_batch()],
    prepare: Some(&PACK_A_PUNCH),
    ..oracle::KIROL_HISTORY_BUFF
};

pub static EXTRA: &[&CardDef] = &[&PACK_A_PUNCH];
