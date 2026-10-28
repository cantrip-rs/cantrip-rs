use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, effects, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target artifact or creature becomes an artifact creature with base power and toughness 4/4 and
/// gains flying. Draw a card." The target keeps what it is and its subtypes (CR 205.1b: the text names no creature type
/// and does not replace any); Artifact and Creature are added (layer 4), flying granted (6) and the base power and
/// toughness set (7b), so counters and pumps count on top. A target no longer legal means the spell does not resolve at
/// all and draws nothing (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let animation = Animation {
        power: 4,
        toughness: 4,
        colors: None,
        types: TypeSet::of(&[CardType::Artifact]),
        subtypes: &[],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::of(&[Keyword::Flying]),
        ability: None,
    };
    animate(g, item.object, permanent, &animation, Duration::EndOfTurn);
    g.end_sentence();
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE], effect),
    ..oracle::I_AM_IRON_MAN
};
