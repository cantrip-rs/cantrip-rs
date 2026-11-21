use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, learn};
use cantrip_oracle as oracle;

/// "The Lion-Turtle can't attack ... unless there are three or more Lesson cards in your graveyard" (CR 508.1c, 205.3k): asked of
/// this creature as attackers are declared, counting its controller's graveyard only (CR 404.1) as it is then. Once declared it
/// stays attacking whatever happens to the graveyard (CR 506.4).
fn attacks_with_three_lessons(q: &dyn Query, asker: &Asker, attacker: ObjectId, _: Target) -> bool {
    attacker != asker.source || learn::lesson_cards_in_graveyard(q, q.controller(asker.source)) >= 3
}

/// "The Lion-Turtle can't ... block unless there are three or more Lesson cards in your graveyard" (CR 509.1b): asked as blockers
/// are declared, the same count.
fn blocks_with_three_lessons(q: &dyn Query, source: ObjectId, blocker: ObjectId, _: ObjectId) -> bool {
    blocker != source || learn::lesson_cards_in_graveyard(q, q.controller(source)) >= 3
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::gain_life_amount::<3>, "When The Lion-Turtle enters, you gain 3 life."),
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
    ],
    attack_rules: &[attacks_with_three_lessons],
    block_rules: &[blocks_with_three_lessons],
    ..oracle::THE_LION_TURTLE
};
