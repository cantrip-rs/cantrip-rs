use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{U}: This creature gains shroud until end of turn and doesn't untap during your next untap step. Tap it." (CR 608.2, 611.2a, 701.26a).
fn creature_gains_shroud_doesnt_untap(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let kind_0_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Shroud]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
            }
        },
    );
    g.end_sentence();
    let objects_1 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| is_live(q, o))
            .collect::<Vec<ObjectId>>()
    };
    let kinds_1 = {
        let mut all = Vec::new();
        for &object in &objects_1 {
            all.push((object, EffectKind::DoesntUntap));
        }
        all
    };
    instruction(
        g,
        |g| {
            for &(object, kind) in &kinds_1 {
                g.add_effect(
                    item.object,
                    object,
                    kind,
                    Duration::DuringNextUntapStepOf(item.controller),
                );
            }
        },
    );
    g.end_sentence();
    let objects_2 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    g.tap_together(&objects_2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{U}"),
            creature_gains_shroud_doesnt_untap,
            "{U}: This creature gains shroud until end of turn and doesn't untap during your next untap step. Tap it.",
        ),
    ],
    ..oracle::HOMARID_WARRIOR
};
