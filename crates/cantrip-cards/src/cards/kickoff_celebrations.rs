use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this enchantment enters, you may discard a card. If you do, draw two cards."
fn may_discard_to_draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_discard_to_draw(g, item, 2)
}

/// "Creatures and Vehicles you control gain haste until end of turn" (CR 611.2c, 702.10): the creatures and the Vehicles, animated or
/// not, its controller controls as it resolves, and nothing that comes later; each once, a Vehicle that is a creature too included.
fn creatures_and_vehicles_gain_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::permanents_you_control_where_gain(g, item, &[Keyword::Haste], |g, id| targets::creature_or_vehicle(g, id));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(THIS_ENTERS, may_discard_to_draw_two, "When this enchantment enters, you may discard a card. If you do, draw two cards.")],
    delayed: &[ability(SACRIFICE_THIS, creatures_and_vehicles_gain_haste, "Sacrifice this enchantment: Creatures and Vehicles you control gain haste until end of turn.")],
    statics: &[max_speed("Max speed \u{2014} Sacrifice this enchantment: Creatures and Vehicles you control gain haste until end of turn.")],
    ..oracle::KICKOFF_CELEBRATIONS
};
