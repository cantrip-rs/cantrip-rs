use cantrip_api::*;
use cantrip_mechanics::{choose, hand, targets, triggers};
use cantrip_oracle as oracle;

/// "You may cast a permanent spell with equal or lesser mana value from your hand without paying its mana cost. If you don't, you
/// may put a land card from your hand onto the battlefield." (CR 608.2d, 608.2g, 118.9, 701.5): the spell that triggered it is
/// read as it last was (CR 608.2h), so an X in it counts as chosen (CR 202.3e); the permanent spell is judged on the face that
/// would be cast and its mana value as that spell has it, X being 0 (CR 107.3b, 709.3a). "If you don't" is a spell not cast, by
/// declining, by none in hand being castable, or by a cast given up (CR 733.1). The land is put onto the battlefield, not played,
/// so it uses no land play (CR 305.2, the ruling).
fn cast_a_permanent_or_put_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    let most = g.mana_value(spell);
    let cast = choose::may_cast_from_hand_free_spell(g, item.controller, item.object, "permanent", |g, way| {
        let types = &g.card(way.face).types;
        targets::PERMANENT_CARD_TYPES.iter().any(|&t| types.has(t)) && way.mana_value <= most
    })?;
    if cast.is_none() {
        hand::may_put_onto_battlefield(g, item.controller, item.object, |c| c.is(CardType::Land))?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_CAST_A_SPELL_FROM_OUTSIDE_YOUR_HAND,
        cast_a_permanent_or_put_a_land,
        "Whenever you cast a spell from anywhere other than your hand, you may cast a permanent spell with equal or lesser mana value from your hand without paying its mana cost. If you don't, you may put a land card from your hand onto the battlefield.",
    )],
    ..oracle::KELLAN_THE_KID
};
