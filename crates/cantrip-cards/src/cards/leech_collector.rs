use cantrip_api::*;
use cantrip_mechanics::prepare::become_prepared;
use cantrip_oracle as oracle;

/// Leech Collector's prepare spell (CR 722): not a card here, only ever a copy. "Each opponent loses 2 life."
pub static BLOODLETTING: CardDef = CardDef {
    name: "Bloodletting",
    cost: mana("{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], each_opponent_loses_2_life),
    text: "Each opponent loses 2 life.",
    ..CardDef::BLANK
};

fn each_opponent_loses_2_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.each_opponent_loses_life(item.controller, 2);
    Ok(())
}

/// "Whenever you gain life for the first time each turn" (CR 119.3, 603.2): life was gained, and none was before it this turn -
/// the history is as it was just before this event, so it holds the earlier gains only. A turn's later gains don't trigger it
/// (CR 603.2d: it is a condition on the trigger, not a limit on how often it triggers).
fn you_gain_life_for_the_first_time_each_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { player, delta } if *delta > 0 && *player == me.controller && q.history().life_gained(me.controller) == 0 => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::LIFE_CHANGE, you_gain_life_for_the_first_time_each_turn),
        become_prepared,
        "Whenever you gain life for the first time each turn, this creature becomes prepared.",
    )],
    prepare: Some(&BLOODLETTING),
    ..oracle::LEECH_COLLECTOR
};

pub static EXTRA: &[&CardDef] = &[&BLOODLETTING];
