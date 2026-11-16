use cantrip_api::*;
use cantrip_mechanics::{choose, history, select};
use cantrip_oracle as oracle;

/// Where the noting ability is in the card's list: "{X}, {T}: ... note the type and amount of mana spent to pay this
/// activation cost", which the mana ability's "last noted type and amount" is linked to (CR 607.2e).
const NOTES: u8 = 0;

/// "Activate only if there are no charge counters on this artifact" (CR 602.5): asked as the ability is activated, never
/// again as it resolves.
fn no_charge_counters(q: &dyn Query, asker: &Asker) -> bool {
    q.state().obj(asker.source).counters(CounterKind::Charge) == 0
}

/// "You may exile a nonland card from your hand. You may cast that card for as long as it remains exiled. Put a charge
/// counter on this artifact and note the type and amount of mana spent to pay this activation cost." The card is exiled as
/// the ability resolves (a countered ability takes nothing, the ruling), with this artifact (CR 607.2a), for the mana
/// ability to name; the permission is to cast it from exile, at the time its type allows and for its costs (the rulings),
/// for as long as that card stays in exile, whatever becomes of this artifact (CR 400.7, 611.2a). The note is made as the
/// ability resolves, with or without a card exiled or a counter put: it is this resolution, which the history keeps
/// (`Event::Resolved`), read back by the mana ability (`history::last_noted_mana`).
fn exile_and_charge(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let hand = g.state().zone(Zone::hand(you)).to_vec();
    let chosen = choose::select(g, you, item.object, Purpose::Exile, hand, select::NONLAND_CARD)?;
    for card in g.exile_with(&chosen, g.linked(item), false) {
        g.allow_play(card, you, PlayGrant::CAST);
    }
    g.add_counters(item.object, CounterKind::Charge, 1);
    Ok(())
}

/// "The last card exiled with this artifact" (CR 607.2a): the card its linked ability last exiled, as the object it became
/// in exile, wherever it has gone since - a card cast from there is the spell that object became (CR 400.7). None ever
/// exiled, none.
fn last_card_exiled(q: &dyn Query, by: Linked) -> ObjectId {
    q.history()
        .all()
        .iter()
        .rev()
        .find_map(|r| match r.event {
            Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Exile) && q.obj(new).exiled.is_some_and(|l| l.is_by_in(q.state(), by)) => Some(new),
            _ => None,
        })
        .unwrap_or(ObjectId::NONE)
}

/// "Add this artifact's last noted type and amount of mana. Spend this mana only to cast the last card exiled with this
/// artifact." (CR 106.6, 605.1a): a mana ability, adding at once the mana last noted, each type in its amount, which may pay
/// only for casting that card (`ManaRestriction::Card`) - its costs whatever they are, additional costs among them (the
/// rulings) - and nothing at all where no card was exiled with it.
fn add_the_last_noted_mana(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let by = g.linked_to(asker);
    let only = ManaRestriction::Card(last_card_exiled(g, by));
    let noted = history::last_noted_mana(g, by, NOTES);
    for m in Mana::ALL {
        let n = noted.get(m);
        if n > 0 {
            g.add_restricted_mana(asker.controller, m, n, only);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{X}").and(TAP),
            exile_and_charge,
            "{X}, {T}: You may exile a nonland card from your hand. You may cast that card for as long as it remains exiled. Put a charge counter on this artifact and note the type and amount of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact.",
        )
        .condition(no_charge_counters),
        mana_ability(
            TAP.and(remove_counters(CounterKind::Charge, 1)),
            add_n(Mana::C, 0).by(add_the_last_noted_mana),
            "{T}, Remove a charge counter from this artifact: Add this artifact's last noted type and amount of mana. Spend this mana only to cast the last card exiled with this artifact.",
        ),
    ],
    ..oracle::ICE_CAULDRON
};
