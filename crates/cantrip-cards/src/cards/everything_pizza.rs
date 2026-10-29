use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{damage_target, effects, lands, targets};
use cantrip_oracle as oracle;

/// "When this artifact enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle"
/// (CR 701.23): not optional, though the player may find nothing.
fn fetch_a_basic(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_hand(g, item.controller, item.object)
}

/// "Target player gains 3 life and draws a card. Each of your opponents discards a card. This artifact deals 3 damage to any
/// target. Put three +1/+1 counters on up to one target creature." (CR 608.2b-c): each target is judged on its own as the
/// ability resolves, one that is no longer legal being left alone while the rest still happens; the sentences are
/// separate, so each ends before the next.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.gain_life(player, 3);
        g.draw(player, 1);
    }
    g.end_sentence();
    effects::each_opponent_discards_a_card(g, item)?;
    g.end_sentence();
    damage_target(g, item, 1, 3);
    g.end_sentence();
    if let Some(creature) = targets::object(g, item, 2) { g.add_counters(creature, CounterKind::PlusOne, 3); }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            fetch_a_basic,
            "When this artifact enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
        ),
        ability(
            cost("{2}{W}{U}{B}{R}{G}, {T}").and(SACRIFICE_THIS),
            effect,
            "{2}{W}{U}{B}{R}{G}, {T}, Sacrifice this artifact: Target player gains 3 life and draws a card. Each of your opponents discards a card. This artifact deals 3 damage to any target. Put three +1/+1 counters on up to one target creature.",
        )
        .targets(&[targets::PLAYER, targets::ANY, up_to_one(targets::CREATURE.filter)]),
    ],
    ..oracle::EVERYTHING_PIZZA
};
