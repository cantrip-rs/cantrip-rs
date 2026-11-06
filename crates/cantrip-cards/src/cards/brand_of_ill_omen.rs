use cantrip_api::*;
use cantrip_mechanics::{targets, cumulative_upkeep};
use cantrip_oracle as oracle;

/// "Enchanted creature's controller can't cast creature spells." (CR 604.1).
fn enchanted_creatures_controller_cant_cast_rule(
    q: &dyn Query,
    asker: &Asker,
    player: PlayerId,
    spell: ObjectId,
) -> bool {
    !(q.attached_to(asker.source).is_some_and(|creature| q.controller(creature) == player)
        && q.chars(spell).is(CardType::Creature))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cumulative_upkeep::Payment::Mana(mana("{R}")),
            "Cumulative upkeep"
        ),
    ],
    enchant: Some(targets::CREATURE),
    spell_rules: &[enchanted_creatures_controller_cant_cast_rule],
    ..oracle::BRAND_OF_ILL_OMEN
};
