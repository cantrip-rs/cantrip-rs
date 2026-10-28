use cantrip_api::*;
use cantrip_mechanics::{Animation, animate};
use cantrip_oracle as oracle;

/// "Until end of turn, Reptil becomes a Dinosaur Hero with base power and toughness [P/T] and gains [keywords]." (CR 611.2a, 613.1d,
/// 613.4b, 205.1a): his creature types are replaced by Dinosaur and Hero (read through the words of this card's text, CR 612) and
/// his base power and toughness set, so counters and pumps still count on top; the keywords are gained. Both abilities used in a turn
/// leave the later one's base power and toughness (CR 613.7b) and both's keywords. The effect ends in the cleanup step (CR 514.2).
fn becomes(g: &mut dyn Game, item: &StackItem, power: i16, toughness: i16, keywords: Keywords) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let animation = Animation {
        power,
        toughness,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[words.subtype(Subtype::Dinosaur), words.subtype(Subtype::Hero)],
        all_creature_types: false,
        replaces_creature_types: true,
        keywords,
        ability: None,
    };
    animate(g, item.object, item.object, &animation, Duration::EndOfTurn);
    Ok(())
}

fn brontosaurus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    becomes(g, item, 3, 5, Keywords::of(&[Keyword::Reach, Keyword::Vigilance]))
}

fn tyrannosaurus_rex(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    becomes(g, item, 6, 6, Keywords::of(&[Keyword::Trample]))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{3}"),
            brontosaurus,
            "Brontosaurus \u{2014} {3}: Until end of turn, Reptil becomes a Dinosaur Hero with base power and toughness 3/5 and gains reach and vigilance.",
        ),
        ability(
            cost("{6}"),
            tyrannosaurus_rex,
            "Tyrannosaurus Rex \u{2014} {6}: Until end of turn, Reptil becomes a Dinosaur Hero with base power and toughness 6/6 and gains trample.",
        ),
    ],
    ..oracle::REPTIL_DINOMORPHER
};
