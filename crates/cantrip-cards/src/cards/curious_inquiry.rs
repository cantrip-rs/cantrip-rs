use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::tokens;
use cantrip_mechanics::{statics, targets, triggers};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a): its controller creates a Clue token.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// The ability the enchanted creature has (CR 613.1f): "Whenever this creature deals combat damage to a player, investigate." It is the
/// creature's own, so its controller investigates, not the Aura's (CR 113.8).
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    delayed: &[triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER, investigate, "Whenever this creature deals combat damage to a player, investigate.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, statics::plus_pt::<1, 1>, "Enchanted creature gets +1/+1."),
        static_ability(
            Layer::Ability,
            attached_object,
            statics::grant_delayed_ability,
            "Enchanted creature has \"Whenever this creature deals combat damage to a player, investigate.\"",
        ),
    ],
    ..oracle::CURIOUS_INQUIRY
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
