use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

const ANIMALS: [Subtype; 18] = [
    Subtype::Spider,
    Subtype::Boar,
    Subtype::Bat,
    Subtype::Bear,
    Subtype::Bird,
    Subtype::Cat,
    Subtype::Dog,
    Subtype::Frog,
    Subtype::Jackal,
    Subtype::Lizard,
    Subtype::Mouse,
    Subtype::Otter,
    Subtype::Rabbit,
    Subtype::Raccoon,
    Subtype::Rat,
    Subtype::Squirrel,
    Subtype::Turtle,
    Subtype::Wolf,
];

/// "Other Spiders, Boars, Bats, ... and Wolves you control" (CR 613.1f, layer 6): every permanent under the source's controller's
/// control but the source itself that has at least one of the eighteen creature types as the earlier layers made it (a changeling
/// among them, CR 702.73a), the types read through the words of the source's text (CR 612.2). It is one ability, so a permanent
/// with several of the types still gets +1/+1 once.
fn other_animals_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let words = q.words(source);
    affected != source
        && q.state().obj(affected).controller == q.state().obj(source).controller
        && ANIMALS.iter().any(|&animal| so_far.has_subtype(words.subtype(animal)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_food_token, "When Spider-Ham enters, create a Food token.")],
    statics: &[static_ability(
        Layer::PtModify,
        other_animals_you_control,
        plus_pt::<1, 1>,
        "Animal May-Ham \u{2014} Other Spiders, Boars, Bats, Bears, Birds, Cats, Dogs, Frogs, Jackals, Lizards, Mice, Otters, Rabbits, Raccoons, Rats, Squirrels, Turtles, and Wolves you control get +1/+1.",
    )],
    ..oracle::SPIDER_HAM_PETER_PORKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
