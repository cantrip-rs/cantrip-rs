use cantrip_api::*;
use cantrip_mechanics::{
    effects::{ask_may, counters_on_self},
    library::exile_top_play_this_turn,
    triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SacrificeToExileTop",
    "sacrifice this creature to exile the top cards of your library, one for each skewer counter on it?",
    "",
    "Sacrifice",
    "Don't",
);

/// "Put a skewer counter on this creature. Then you may sacrifice it. If you do, exile the top X cards of your library, where X is
/// the number of skewer counters on this creature. You may play those cards this turn" (CR 122.1, 608.2c, 608.2h, 701.21a, 701.13a):
/// the counter goes on first and the choice is made as the ability resolves. X is the number of counters it had on the battlefield,
/// counted before the sacrifice (its last known information), and the cards are the ability controller's own.
fn put_a_skewer_counter_then_may_sacrifice_to_exile_that_many(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counters_on_self::<{ CounterKind::Skewer as usize }, 1>(g, item)?;
    g.end_sentence();
    if g.can_sacrifice(item.controller, item.object) && ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        let skewers = g.obj(item.object).counters(CounterKind::Skewer);
        if !g.sacrifice(item.controller, &[item.object]).is_empty() {
            g.end_sentence();
            exile_top_play_this_turn(g, item.controller, usize::try_from(skewers).unwrap_or_default());
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
        put_a_skewer_counter_then_may_sacrifice_to_exile_that_many,
        "Whenever this creature deals combat damage to a player, put a skewer counter on this creature. Then you may sacrifice it. If you do, exile the top X cards of your library, where X is the number of skewer counters on this creature. You may play those cards this turn.",
    )],
    ..oracle::ROTISSERIE_ELEMENTAL
};
