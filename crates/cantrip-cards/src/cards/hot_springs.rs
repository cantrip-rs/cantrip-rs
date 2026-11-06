use cantrip_api::*;
use cantrip_mechanics::{attach, prevention, statics, targets};
use cantrip_oracle as oracle;

/// Prevent the next 1 damage to the target this turn (CR 615.7).
fn shield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_damage_to_target(g, item, 0, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        attach::attached_object,
        statics::grant_delayed_ability,
        "Enchanted land has \"{T}: Prevent the next 1 damage that would be dealt to any target this turn.\"",
    )],
    delayed: &[ability(
        TAP,
        shield,
        "{T}: Prevent the next 1 damage that would be dealt to any target this turn.",
    ).targets(&[targets::ANY])],
    enchant: Some(targets::LAND_YOU_CONTROL),
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 1 damage that would be dealt to any target this turn.")],
    ..oracle::HOT_SPRINGS
};
