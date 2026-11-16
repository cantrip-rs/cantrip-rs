use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, triggers};
use cantrip_oracle as oracle;

/// "Return another creature you control to its owner's hand. If you do, put a +1/+1 counter on this creature. Otherwise, you may
/// return this creature to its owner's hand." (CR 608.2c, 608.2d): nothing is targeted. As the ability resolves its
/// controller must return one of their other creatures if they control any (ruling 2024-07-26), chosen then (exactly one); the
/// counter comes only if a creature really arrived in a hand ("if you do"). Otherwise (no other creature, or a commander that went
/// to the command zone instead, CR 903.9b) they may return the Elder itself, a yes-or-no choice (CR 608.2d). Nothing happens to an Elder that has left the
/// battlefield (CR 400.7).
fn return_another_or_maybe_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let permanents = g.permanents_of(item.controller);
    let rule = select::another_creature_permanents(select::exactly(1));
    // The rule says whether there is another creature to return; with none, nothing is returned and "otherwise" applies.
    let returned = if cantrip_api::select::first_answer(&*g, &rule, item.controller, item.object, &permanents, true).is_some_and(|cards| !cards.is_empty()) {
        let chosen = choose::select(g, item.controller, item.object, Purpose::ReturnToHand, permanents, rule)?;
        // A commander its owner puts into the command zone instead (CR 903.9b) was not returned to a hand.
        g.return_to_hand(&chosen).into_iter().any(|new| g.obj(new).zone.kind() == ZoneKind::Hand)
    } else {
        false
    };
    if returned {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    } else if effects::ask_may(g, item.controller, item.object, Purpose::ReturnToHand)? {
        g.return_to_hand(&[item.object]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        return_another_or_maybe_this,
        "At the beginning of your upkeep, return another creature you control to its owner's hand. If you do, put a +1/+1 counter on this creature. Otherwise, you may return this creature to its owner's hand.",
    )],
    ..oracle::MISTBREATH_ELDER
};
