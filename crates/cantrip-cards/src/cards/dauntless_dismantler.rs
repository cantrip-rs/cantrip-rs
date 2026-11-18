use cantrip_api::*;
use cantrip_mechanics::replacements::{enter_tapped, opponents_enter_tapped};
use cantrip_oracle as oracle;

/// "Destroy each artifact with mana value X" (CR 701.8a, 202.3): every artifact on the battlefield, whoever controls it, whose
/// mana value is exactly the X announced for the ability ({X}{X}{W} counted it twice, CR 107.3), destroyed at once. An artifact
/// with {X} in its mana cost counts that X as 0 on the battlefield, whatever it was cast with (CR 202.3e), and a token with no
/// mana cost has mana value 0 (CR 202.3a).
fn destroy_artifacts_with_mana_value_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let doomed: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Artifact) && g.mana_value(id) == u32::from(item.x))
        .collect();
    g.destroy_all(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    replacements: &[opponents_enter_tapped::<{ enter_tapped::ARTIFACTS }>("enters tapped")],
    abilities: &[ability(cost("{X}{X}{W}").and(SACRIFICE_THIS), destroy_artifacts_with_mana_value_x, "{X}{X}{W}, Sacrifice this creature: Destroy each artifact with mana value X.")],
    ..oracle::DAUNTLESS_DISMANTLER
};
