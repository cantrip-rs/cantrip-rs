use cantrip_api::*;
use cantrip_mechanics::{animate, targets, Animation};
use cantrip_oracle as oracle;

/// "Until end of turn, target artifact or creature becomes a Dinosaur artifact creature with base power and toughness 4/3 in addition to
/// its other types." (CR 205.1b, 611.2a, 613.1d, 613.4b): Artifact, Creature and Dinosaur (read through the spell's words, CR 612) are added
/// to what it is (layer 4) and its base power and toughness are set to 4/3 (7b), so counters and pumps count on top. A target no longer
/// legal as the spell resolves does nothing (CR 608.2b).
fn becomes_a_dinosaur_artifact_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let dinosaur = [g.words(item.object).subtype(Subtype::Dinosaur)];
    let animation = Animation {
        power: 4,
        toughness: 3,
        colors: None,
        types: TypeSet::of(&[CardType::Artifact]),
        subtypes: &dinosaur,
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::NONE,
        ability: None,
    };
    animate(g, item.object, permanent, &animation, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE], becomes_a_dinosaur_artifact_creature),
    ..oracle::RELICS_ROAR
};
