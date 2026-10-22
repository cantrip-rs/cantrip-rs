use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

const NAME: &str = "\"Name Sticker\" Goblin";

/// "When this creature enters from anywhere other than a graveyard or exile" (CR 603.6a): the entry event's own source
/// zone, which is what stops a reanimated or blinked Goblin paying for itself again.
fn enters_not_from_graveyard_or_exile(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, from, to, .. }
            if *new == source && to.is(ZoneKind::Battlefield) && !from.is(ZoneKind::Graveyard) && !from.is(ZoneKind::Exile) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "If it's on the battlefield and you control 9 or fewer creatures named "Name Sticker" Goblin" (CR 603.4): checked
/// as the ability would trigger and again as it resolves. Counted off the board as it stands.
fn on_battlefield_and_nine_or_fewer(q: &dyn Query, asker: &Asker) -> bool {
    if !targets::permanent(q, asker.source) {
        return false;
    }
    let named = q
        .permanents_of(asker.controller)
        .into_iter()
        // A creature's names as it has them now (CR 201.2a): a copy's, or one an effect named (CR 612.8).
        .filter(|&id| q.chars(id).is(CardType::Creature) && library::object_names(q, id).iter().any(|name| *name == NAME))
        .count();
    named <= 9
}

/// Roll a 20-sided die. 1-6: add {R}{R}{R}{R}. 7-14: add {R}{R}{R}{R}{R}. 15-20: add {R}{R}{R}{R}{R}{R}.
fn roll(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rolled = g.roll_die(item.controller, 20).result;
    let mana = if rolled <= 6 {
        4
    } else if rolled <= 14 {
        5
    } else {
        6
    };
    g.add_mana(item.controller, Mana::R, mana);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, enters_not_from_graveyard_or_exile), roll, "When this creature enters from anywhere other than a graveyard or exile, if it's on the battlefield and you control 9 or fewer creatures named \"Name Sticker\" Goblin, roll a 20-sided die. 1-6 | Add {R}{R}{R}{R}. 7-14 | Add {R}{R}{R}{R}{R}. 15-20 | Add {R}{R}{R}{R}{R}{R}.")
        .condition(on_battlefield_and_nine_or_fewer)],
    ..oracle::NAME_STICKER_GOBLIN
};
