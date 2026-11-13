use cantrip_api::*;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature with toughness 4 or greater. You gain 1 life." (CR 701.8, 608.2b): with its one target illegal
/// the core does not resolve the spell at all, so no life; with a legal target the life is gained whether or not the destroy
/// did anything (an indestructible target, a regeneration shield), the instructions being independent (CR 608.2c).
fn destroy_and_gain_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    effects::gain_life(g, item, 1)
}

static MODAL: ModalDef = modes(&[
    mode(destroy_and_gain_one, "Destroy target creature with toughness 4 or greater. You gain 1 life.")
        .targets(&[targets::CREATURE_WITH_TOUGHNESS_4_OR_GREATER]),
    mode(effects::draw_a_card_and_gain_life::<2>, "You draw a card and gain 2 life."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SURGICAL_PRECISION };
