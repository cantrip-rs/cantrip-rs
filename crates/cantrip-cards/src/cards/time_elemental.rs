use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, triggers::THIS_ATTACKS_OR_BLOCKS};
use cantrip_oracle as oracle;

/// "At end of combat, sacrifice it and it deals 5 damage to you" (CR 701.21a, 113.7a).
fn delayed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(source) = item.subject else { return Ok(()) };
    instruction(g, |g| {
        g.sacrifice(item.controller, &[source]);
        g.damage_together(&[(source, Target::Player(item.controller), 5)]);
    });
    Ok(())
}

/// "Return target permanent that isn't enchanted to its owner's hand" (CR 115.1).
fn unenchanted_permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && !targets::is_enchanted(q, id))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_OR_BLOCKS,
            delayed::at_end_of_combat_about_this,
            "When this creature attacks or blocks, at end of combat, sacrifice it and it deals 5 damage to you.",
        ),
        ability(
            cost("{2}{U}{U}").and(TAP),
            effects::return_first_target_to_hand,
            "{2}{U}{U}, {T}: Return target permanent that isn't enchanted to its owner's hand.",
        ).targets(&[target(unenchanted_permanent)]),
    ],
    delayed: &[delayed_trigger(
        delayed,
        "When this creature attacks or blocks, at end of combat, sacrifice it and it deals 5 damage to you.",
    )],
    ..oracle::TIME_ELEMENTAL
};
