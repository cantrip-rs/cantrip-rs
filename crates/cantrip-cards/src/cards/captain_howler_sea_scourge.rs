use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+0 until end of turn for each card discarded this way. Whenever that creature deals combat
/// damage to a player this turn, you draw a card": the count is the batch the trigger saw (CR 603.2c), and the delayed
/// ability (CR 603.7b) is fixed on that creature, which is its source, so its trigger asks only whether that creature dealt
/// combat damage to a player. A target that is no longer legal gets neither (CR 608.2b).
fn pump_and_watch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let discarded = g.event_objects(item).len() as i32;
    g.add_effect(item.object, creature, EffectKind::Pump(2 * discarded, 0), Duration::EndOfTurn);
    delayed::this_turn(g, item, creature);
    Ok(())
}

fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward_mana_and_life::<2, 2>("Ward\u{2014}{2}, Pay 2 life. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2} and 2 life.)"),
        triggered(triggers::YOU_DISCARD, pump_and_watch, "Whenever you discard one or more cards, target creature gets +2/+0 until end of turn for each card discarded this way. Whenever that creature deals combat damage to a player this turn, you draw a card.")
            .once_per_batch()
            .targets(&[targets::CREATURE]),
    ],
    delayed: &[triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER, draw_a_card, "Whenever that creature deals combat damage to a player this turn, you draw a card.")],
    ..oracle::CAPTAIN_HOWLER_SEA_SCOURGE
};
