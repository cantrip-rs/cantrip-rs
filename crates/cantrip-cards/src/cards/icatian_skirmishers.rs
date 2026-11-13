use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_SUBJECT;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, all creatures banded with it gain first strike until end of turn." (CR 608.2, 611.2a).
/// Its band is asked as it resolves, of its last-known information if it has left (CR 608.2h).
fn all_creatures_banded_gain_first(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        let banded = item.subject.object().map_or_else(Vec::new, |it| q.creatures_banded_with(it));
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| q.chars(id).is(CardType::Creature) && banded.contains(&id))
            .collect::<Vec<ObjectId>>()
    };
    let kind_0_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::FirstStrike]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_SUBJECT,
            all_creatures_banded_gain_first,
            "Whenever this creature attacks, all creatures banded with it gain first strike until end of turn.",
        ),
    ],
    ..oracle::ICATIAN_SKIRMISHERS
};
