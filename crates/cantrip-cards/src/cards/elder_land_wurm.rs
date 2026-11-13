use cantrip_api::*;
use cantrip_oracle as oracle;

/// "When this creature blocks, it loses defender." (CR 608.2, 611.2a).
fn loses_defender(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = item.subject.object().into_iter().collect::<Vec<ObjectId>>();
    let kind_0_0_0 = EffectKind::LoseKeywords(Keywords::of(&[Keyword::Defender]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::Indefinite);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            cantrip_mechanics::triggers::THIS_BLOCKS,
            loses_defender,
            "When this creature blocks, it loses defender.",
        ),
    ],
    ..oracle::ELDER_LAND_WURM
};
