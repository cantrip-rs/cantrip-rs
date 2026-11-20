use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{Animation, effects, tokens};
use cantrip_oracle as oracle;

/// "Target noncreature artifact you control" (CR 115.1, 110.4): an artifact on the battlefield that is not also a creature, as the
/// layers have it, controlled by the ability's controller.
fn noncreature_artifact_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.controller(id) == asker.controller && {
        let chars = q.chars(id);
        chars.is(CardType::Artifact) && !chars.is(CardType::Creature)
    })
}

/// "Until end of turn, target noncreature artifact you control becomes a 4/4 artifact creature." It is an artifact already and
/// keeps all it is (CR 205.1b): Creature is added to its types (layer 4) and its base power and toughness are set (layer 7b), so
/// counters and pumps count on top. The text names no color and no creature type, so it changes neither. A target that has left
/// the battlefield, or is illegal, is not affected (CR 608.2b, 400.7).
fn becomes_a_4_4_artifact_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(artifact) = cantrip_mechanics::targets::object(g, item, 0) else { return Ok(()) };
    let animation = Animation {
        power: 4,
        toughness: 4,
        colors: None,
        types: TypeSet::of(&[CardType::Artifact]),
        subtypes: &[],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::NONE,
        ability: None,
    };
    cantrip_mechanics::animate(g, item.object, artifact, &animation, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token."),
        ability(
            cost("{2}{G}"),
            becomes_a_4_4_artifact_creature,
            "{2}{G}: Until end of turn, target noncreature artifact you control becomes a 4/4 artifact creature.",
        )
        .targets(&[target(noncreature_artifact_you_control)]),
        ability(cost("{2}, {T}").and(SACRIFICE_THIS), effects::gain_three_life, "{2}, {T}, Sacrifice this creature: You gain 3 life."),
    ],
    ..oracle::TOUGH_COOKIE
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
