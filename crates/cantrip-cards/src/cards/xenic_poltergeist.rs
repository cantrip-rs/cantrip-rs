use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Until your next upkeep, target noncreature artifact becomes an artifact creature with power and toughness each equal to its mana value." (CR 608.2h, 611.2a).
fn animate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = targets::object(g, item, 0) else { return Ok(()) };
    // Registered fixed costs and two-face combinations fit i32; battlefield X is zero.
    let Ok(value) = i32::try_from(g.mana_value(object)) else { return Ok(()) };
    let duration = Duration::UntilNextStepOf { step: Step::Upkeep, player: Some(item.controller) };
    instruction(g, |g| {
        g.add_effect(item.object, object, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), duration);
        g.add_effect(item.object, object, EffectKind::SetPt(value, value), duration);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, animate, "{T}: Until your next upkeep, target noncreature artifact becomes an artifact creature with power and toughness each equal to its mana value.")
            .targets(&[targets::NONCREATURE_ARTIFACT]),
    ],
    ..oracle::XENIC_POLTERGEIST
};
