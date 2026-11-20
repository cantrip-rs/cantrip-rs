use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::{self, FOOD};
use cantrip_oracle as oracle;

/// "Destroy up to one target ... Create a Food token." (CR 701.8a): two sentences; the token comes whether or not anything was
/// targeted, or destroyed. If the one target chosen is illegal as it resolves, nothing happens (CR 608.2b).
fn destroy_then_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(target) = targets::object(g, item, 0) {
        g.destroy(target);
    }
    g.end_sentence();
    tokens::create_a_food_token(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_ENCHANTMENT_OR_CREATURE_WITH_FLYING.optional()], destroy_then_food),
    ..oracle::SPIDER_FOOD
};

/// The Food token it makes.
pub static EXTRA: &[&CardDef] = &[&FOOD];
