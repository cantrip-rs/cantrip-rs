use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// The entry replacement exiles creature cards and gives a counter for each (CR 614.1c, 122.6, 701.13a).
fn as_creature_enters_exile_x_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let asker = Asker::new(entering.object, entering.controller);
    let cards = g.state().zone(Zone::graveyard(entering.controller)).iter().rev().copied()
        .filter(|&id| targets::card_in_your_graveyard(g, &asker, Target::Object(id), &[]))
        .collect();
    let selected = choose::select(
        g,
        entering.controller,
        entering.object,
        Purpose::Exile,
        cards,
        select::creature_cards(select::as_many_as_can(entering.x)),
    )?;
    let moved = g.exile_all(&selected);
    let exiled = moved.iter().filter(|&&id| {
        g.state().obj(id).zone.kind() == ZoneKind::Exile
    }).count();
    if exiled != entering.x as usize {
        entering.instead = Some(Zone::graveyard(g.obj(entering.object).owner));
    } else {
        let kinds = [CounterKind::Plus2Plus0, CounterKind::PlusOne, CounterKind::Plus0Plus2];
        for _ in 0..exiled {
            let picked = choose::one_of(
                g, entering.controller, entering.object,
                kinds.iter().copied().map(Label::Counter).collect(),
            )?;
            entering.counters[kinds[picked as usize] as usize] += 1;
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(as_creature_enters_exile_x_enters),
    ..oracle::FRANKENSTEINS_MONSTER
};
