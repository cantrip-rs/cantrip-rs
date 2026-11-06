use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "As this creature enters, it becomes your choice of a 3/3 artifact creature, a 2/2 artifact creature with flying, or a 1/6 Wall artifact creature with defender in addition to its other types. (A creature with defender can't attack.)" (CR 614.1c).
fn as_creature_enters_becomes_choice_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    choose::option_as_enters(g, entering, &CARD, 0..3)?;
    match entering.chosen_option.map(|chosen| chosen.index) {
        Some(0) => {
            entering.copy = Some({
                let q: &dyn Query = g;
                entering
                    .copy
                    .unwrap_or_else(|| q.copiable_values(entering.object))
                    .sized(3, 3)
                    .with_type(CardType::Artifact)
                    .with_type(CardType::Creature)
            });
        }
        Some(1) => {
            entering.copy = Some({
                let q: &dyn Query = g;
                entering
                    .copy
                    .unwrap_or_else(|| q.copiable_values(entering.object))
                    .sized(2, 2)
                    .with_type(CardType::Artifact)
                    .with_type(CardType::Creature)
                    .with_keyword(Keyword::Flying)
            });
        }
        _ => {
            entering.copy = Some({
                let q: &dyn Query = g;
                entering
                    .copy
                    .unwrap_or_else(|| q.copiable_values(entering.object))
                    .sized(1, 6)
                    .with_subtype(Subtype::Wall)
                    .with_type(CardType::Artifact)
                    .with_type(CardType::Creature)
                    .with_keyword(Keyword::Defender)
            });
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(as_creature_enters_becomes_choice_enters),
    labels: &[
        "3/3 artifact creature",
        "2/2 artifact creature with flying",
        "1/6 Wall artifact creature with defender",
    ],
    ..oracle::PRIMAL_CLAY
};
