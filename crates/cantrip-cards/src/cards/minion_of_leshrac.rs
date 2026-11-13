use cantrip_api::*;
use cantrip_mechanics::{effects, protection, select, targets, triggers};
use cantrip_oracle as oracle;

/// The upkeep payment is optional; only damage actually dealt taps the Minion (CR 608.2, 120.3, 701.21a).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let source = item.object;
    let paid = !effects::may_sacrifice(g, you, source, select::another_creature_permanents(select::up_to(1)))?.is_empty();
    g.end_sentence();
    if !paid {
        let before = g.history().head() as usize;
        g.damage_together(&[(source, Target::Player(you), 5)]);
        let dealt = g.history().all()[before..].iter().any(|r| {
            matches!(r.event, Event::Damage { source: s, target: Target::Player(p), amount, .. }
                if s == source && p == you && amount > 0)
        });
        g.end_sentence();
        if dealt {
            g.tap(source);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            upkeep,
            "At the beginning of your upkeep, this creature deals 5 damage to you unless you sacrifice a creature other than this creature. If this creature deals damage to you this way, tap it.",
        ),
        ability(TAP, effects::destroy_first_target, "{T}: Destroy target creature or land.")
            .targets(&[targets::CREATURE_OR_LAND]),
    ],
    statics: &[protection::from::<{ Color::Black as u8 }>("Protection from black")],
    ..oracle::MINION_OF_LESHRAC
};
