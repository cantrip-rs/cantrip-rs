use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, lasting_rules, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target land you control becomes a 4/4 Dinosaur creature with reach and haste. It's still a land. It must be
/// blocked this turn if able." Three sentences, each its own instruction (CR 608.2c): the animation (CR 613.1d, 613.4b: Dinosaur and
/// the creature type added, the 4/4 set in layer 7b, reach and haste granted, its colors untouched since the text names none), then the
/// requirement, a rule of the game on that one object for the turn (CR 509.1c, 611.2c) and not a characteristic. Nothing happens if
/// the land is no longer a legal target as the spell resolves (CR 608.2b).
fn land_becomes_a_dinosaur_that_must_be_blocked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    let animation = Animation {
        power: 4,
        toughness: 4,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[g.words(item.object).subtype(Subtype::Dinosaur)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::of(&[Keyword::Reach, Keyword::Haste]),
        ability: None,
    };
    animate(g, item.object, land, &animation, Duration::EndOfTurn);
    g.end_sentence();
    g.add_lasting_rule(item, 0, Target::Object(land), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::LAND_YOU_CONTROL], land_becomes_a_dinosaur_that_must_be_blocked),
    lasting_rules: &[LastingRule {
        text: "It must be blocked this turn if able.",
        rule: LastingRuleKind::MustBeBlocked,
        applies: lasting_rules::the_object_named,
    }],
    ..oracle::DISTURBED_SLUMBER
};
