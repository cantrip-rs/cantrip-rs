use cantrip_api::*;
use cantrip_mechanics::library::look_at_top_take_some_rest_to_graveyard;
use cantrip_mechanics::players;
use cantrip_mechanics::targets::filters::creature;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library. Put two of them into your hand and the rest into your graveyard. You lose 2
/// life." (CR 701.20e, 400.3, 119.3): `look_at_top_take_some_rest_to_graveyard`, then the life.
fn look_at_four_and_lose_two_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top_take_some_rest_to_graveyard(g, item.controller, item.object, 4, 2)?;
    g.lose_life(item.controller, 2);
    Ok(())
}

/// "Each opponent loses 3 life unless they discard a card or sacrifice a creature"
/// (`players::each_opponent_loses_life_unless_sacrifices_or_discards`, once).
fn punish_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    players::each_opponent_loses_life_unless_sacrifices_or_discards(g, item, &CARD, creature, 1, 3)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, look_at_four_and_lose_two_life, "When this artifact enters, look at the top four cards of your library. Put two of them into your hand and the rest into your graveyard. You lose 2 life."),
        ability(
            cost("{2}, {T}").and(Cost { collect_evidence: 3, ..Cost::NONE }),
            punish_each_opponent,
            "{2}, {T}, Collect evidence 3: Each opponent loses 3 life unless they discard a card or sacrifice a creature.",
        ),
    ],
    labels: &["Sacrifice a creature", "Discard a card", "Lose 3 life"],
    ..oracle::POLYGRAPH_ORB
};
