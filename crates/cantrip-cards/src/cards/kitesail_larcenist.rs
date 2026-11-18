use cantrip_api::*;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_oracle as oracle;

fn other_artifact_or_creature_the_slots_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    id != asker.source && targets::controlled_by_slot_opponent(q, id) && filters::artifact_or_creature(q, asker, id)
}

fn other_artifact_or_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    id != asker.source && q.controller(id) == asker.controller && targets::artifact_or_creature_filter(q, asker, t)
}

/// "For each player" is the controller (a slot of their own) and each opponent (a slot each, in seat order) (CR 601.2c, 115.1).
const YOUR_OWN: TargetSpec = target(other_artifact_or_creature_you_control).between(0, 1).labelled("Artifact or creature you control");
const FOR_EACH_OPPONENT: TargetSpec =
    target(other_artifact_or_creature_the_slots_opponent_controls).between(0, TargetSpec::EACH_OPPONENT).labelled("Artifact or creature {player} controls");

/// "For as long as this creature remains on the battlefield, the chosen permanents become Treasure artifacts with '{T}, Sacrifice this
/// artifact: Add one mana of any color' and lose all other abilities" (CR 611.2a, 613.1d, 613.1f, 613.7, 205.1a): a Treasure artifact has
/// artifact as its only card type (so a creature's types go with its creature type, CR 205.3d) and Treasure as its only artifact type; it
/// loses its abilities in the layer before the mana ability comes (CR 613.7). If the Larcenist has left the battlefield by the time this
/// resolves, the duration has already ended and nothing happens (ruling, CR 611.2b).
fn become_treasures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.is_on_battlefield(item.object) {
        return Ok(());
    }
    let objects = targets::legal_objects(g, item);
    let card = g.card_id(&CARD);
    let until = Duration::WhileSourceOnBattlefield;
    instruction(g, |g| {
        for &o in &objects {
            g.add_effect(item.object, o, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])), until);
            g.add_effect(item.object, o, EffectKind::LoseSubtypesOf(CardType::Artifact), until);
            g.add_effect(item.object, o, EffectKind::AddSubtype(Subtype::Treasure), until);
        }
    });
    g.end_sentence();
    instruction(g, |g| {
        for &o in &objects {
            g.add_effect(item.object, o, EffectKind::LoseAllAbilities, until);
            g.add_effect(item.object, o, EffectKind::AddAbility(AbilityRef::delayed(card, 0)), until);
        }
    });
    Ok(())
}

// Flying is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)"),
        triggered(
            THIS_ENTERS,
            become_treasures,
            "When this creature enters, for each player, choose up to one other target artifact or creature that player controls. For as long as this creature remains on the battlefield, the chosen permanents become Treasure artifacts with \"{T}, Sacrifice this artifact: Add one mana of any color\" and lose all other abilities.",
        )
        .targets(&[YOUR_OWN, FOR_EACH_OPPONENT]),
    ],
    delayed: tokens::TREASURE.abilities,
    ..oracle::KITESAIL_LARCENIST
};
