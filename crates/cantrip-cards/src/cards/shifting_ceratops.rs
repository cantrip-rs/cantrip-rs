use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "{G}: This creature gains your choice of reach, trample, or haste until end of turn." (CR 608.2, 611.2a).
fn creature_gains_choice_reach_trample(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let chosen_0 = choose::option(
        g,
        item.controller,
        item.object,
        item.ability.card,
        0..3,
    )? - 0;
    g.end_sentence();
    let objects_1 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_1_0_0 = match chosen_0 {
        0 => EffectKind::AddKeywords(Keywords::of(&[Keyword::Reach])),
        1 => EffectKind::AddKeywords(Keywords::of(&[Keyword::Trample])),
        _ => EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])),
    };
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                g.add_effect(item.object, object, kind_1_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    protection: ColorSet(2),
    abilities: &[
        ability(
            cost("{G}"),
            creature_gains_choice_reach_trample,
            "{G}: This creature gains your choice of reach, trample, or haste until end of turn.",
        ),
    ],
    labels: &["reach", "trample", "haste"],
    ..oracle::SHIFTING_CERATOPS
};
