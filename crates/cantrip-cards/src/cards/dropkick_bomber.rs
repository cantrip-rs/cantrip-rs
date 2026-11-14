use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DEALS_COMBAT_DAMAGE;
use cantrip_mechanics::{effects, lands, statics, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, another target Goblin you control gains flying and \"When this creature deals combat damage, sacrifice it.\""
/// (CR 611.2a, 613.1f, 702.9b): continuous effects fixed to the Goblin as the ability resolves (CR 611.2c), granting the keyword and the
/// triggered ability in layer 6 - the creature's own, so its controller sacrifices it. A target no longer legal gets nothing (CR 608.2b).
fn grant_flying_and_sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(goblin) = targets::object(g, item, 0) else { return Ok(()) };
    let this = g.card_id(&CARD);
    instruction(g, |g| {
        effects::gain_until_end_of_turn(g, item.object, goblin, Keyword::Flying);
        g.add_effect(item.object, goblin, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Goblin) }>,
        statics::plus_pt::<1, 1>,
        "Other Goblins you control get +1/+1.",
    )],
    abilities: &[ability(
        cost("{R}"),
        grant_flying_and_sacrifice,
        "{R}: Until end of turn, another target Goblin you control gains flying and \"When this creature deals combat damage, sacrifice it.\"",
    )
    .targets(&[target(targets::another_permanent_of_subtype_you_control::<{ lands::sub(Subtype::Goblin) }>)])],
    delayed: &[triggered(THIS_DEALS_COMBAT_DAMAGE, effects::sacrifice_self, "When this creature deals combat damage, sacrifice it.").once_per_batch()],
    ..oracle::DROPKICK_BOMBER
};
