use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_oracle as oracle;

/// "{T}: Target creature loses first strike or swampwalk until end of turn." (CR 608.2, 611.2a).
fn target_creature_loses_first_strike(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let chosen_1 = choose::option(
        g,
        item.controller,
        item.object,
        item.ability.card,
        0..2,
    )? - 0;
    g.end_sentence();
    let objects_2 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .collect::<Vec<_>>();
    let kind_2_0_0 = {
        let q: &dyn Query = g;
        match chosen_1 {
            0 => EffectKind::LoseKeywords(Keywords::of(&[Keyword::FirstStrike])),
            _ => {
                EffectKind::LoseKeywords(
                    Keyword::landwalk(q.words(item.object).subtype(Subtype::Swamp))
                        .map_or(Keywords::NONE, |walk| Keywords::of(&[walk])),
                )
            }
        }
    };
    instruction(
        g,
        |g| {
            for &object in &objects_2 {
                g.add_effect(item.object, object, kind_2_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::B), "{T}: Add {B}."),
        ability(
                TAP,
                target_creature_loses_first_strike,
                "{T}: Target creature loses first strike or swampwalk until end of turn.",
            )
            .targets(&[targets::CREATURE]),
    ],
    labels: &["first strike", "swampwalk"],
    ..oracle::URBORG
};
