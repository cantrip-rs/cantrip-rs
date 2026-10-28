use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers};
use cantrip_oracle as oracle;

/// "Discard a card, then draw a card. Then put a +1/+1 counter on that creature for each card you've discarded
/// this turn" Count discards after this discard; draw even if the attacker has left.
fn discard_draw_then_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::discard_then_draw(g, item, 1)?;
    let Target::Object(attacker) = item.subject else { return Ok(()) };
    let discarded = history::cards_discarded_this_turn(g, item.controller) as i16;
    g.add_counters(attacker, CounterKind::PlusOne, discarded);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::ATTACKS, triggers::creature_you_control_attacks_a_player_alone),
        discard_draw_then_grow,
        "Whenever a creature you control attacks a player alone, discard a card, then draw a card. Then put a +1/+1 counter on that creature for each card you've discarded this turn.",
    )],
    ..oracle::JIANG_YANGGU_ALONE
};
