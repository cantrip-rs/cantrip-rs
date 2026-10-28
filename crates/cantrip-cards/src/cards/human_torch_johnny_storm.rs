use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_DRAW_A_CARD;
use cantrip_mechanics::{damage_target, power_up, targets};
use cantrip_oracle as oracle;

/// "If you control another Hero" (CR 603.4): a permanent of its controller's other than this one with the Hero subtype, read
/// through the words of this card's text (CR 612); checked as the ability would trigger and again as it resolves.
fn control_another_hero(q: &dyn Query, asker: &Asker) -> bool {
    let hero = q.words(asker.source).subtype(Subtype::Hero);
    q.permanents_of(asker.controller).into_iter().any(|id| id != asker.source && q.chars(id).has_subtype(hero))
}

/// "Human Torch deals 1 damage to target opponent."
fn ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 1);
    Ok(())
}

/// "Put three +1/+1 counters on Human Torch": on nothing if he has left the battlefield (CR 400.7).
fn three_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOU_DRAW_A_CARD, ping, "Whenever you draw a card, if you control another Hero, Human Torch deals 1 damage to target opponent.")
            .targets(&[targets::OPPONENT])
            .condition(control_another_hero),
        power_up::ability(
            cost("{6}{R}"),
            three_counters,
            "Power-up \u{2014} {6}{R}: Put three +1/+1 counters on Human Torch. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
        ),
    ],
    ..oracle::HUMAN_TORCH_JOHNNY_STORM
};
