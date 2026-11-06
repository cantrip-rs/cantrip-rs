use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Put a soul counter on this land. Then if there are three or more soul counters on it, remove those counters, transform
/// it, then untap it." The counters are counted as the first sentence leaves them (CR 608.2c), so a replacement that puts
/// more counts too. The transform is the ability's own instruction about its source, which does nothing if the permanent
/// has transformed since the ability was activated (CR 701.27f); the untap still happens. An instruction about a
/// permanent that has left the battlefield does nothing to it (CR 400.7).
fn soul_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hostel = item.object;
    g.add_counters(hostel, CounterKind::Soul, 1);
    g.end_sentence();
    let souls = g.obj(hostel).counters(CounterKind::Soul);
    if souls < 3 {
        return Ok(());
    }
    g.remove_counters(hostel, CounterKind::Soul, souls);
    g.transform_by(hostel, item)?;
    g.untap(hostel);
    Ok(())
}

/// "You may exile a creature card from your graveyard. If you do, each opponent loses X life and you gain X life, where X
/// is the number of creature cards exiled with this creature." Choosing none is not doing it (CR 608.2d). The card is
/// exiled with this creature (CR 607.2a), so X counts it among the creature cards this object has exiled, as they are in
/// exile now; a Creeping Inn that phased out and in is the same object and still has them (the 2021-09-24 ruling).
fn exile_and_drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let graveyard: Vec<ObjectId> = g.state().zone(Zone::graveyard(you)).to_vec();
    let chosen = choose::select(g, you, item.object, Purpose::Exile, graveyard, select::CREATURE_CARD)?;
    if g.exile_with(&chosen, g.linked(item), false).is_empty() {
        return Ok(());
    }
    g.end_sentence();
    let x = g.exiled_with(g.linked(item)).into_iter().filter(|&card| g.chars(card).is(CardType::Creature)).count() as i32;
    g.each_opponent_loses_life(you,(x) as i32);
    g.gain_life(you,(x) as i32);
    Ok(())
}

/// "This creature phases out" (CR 702.26b): it phases in during its controller's next untap step.
fn phase_out(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.phase_out(&[item.object]);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, exile_and_drain, "Whenever this creature attacks, you may exile a creature card from your graveyard. If you do, each opponent loses X life and you gain X life, where X is the number of creature cards exiled with this creature."),
        ability(cost("{4}"), phase_out, "{4}: This creature phases out."),
    ],
    ..oracle::CREEPING_INN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{1}, {T}").and(sacrifice(1, targets::filters::creature)), soul_counter, "{1}, {T}, Sacrifice a creature: Put a soul counter on this land. Then if there are three or more soul counters on it, remove those counters, transform it, then untap it. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::HOSTILE_HOSTEL
};
