use cantrip_api::*;
use cantrip_mechanics::{library, select, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// The three cards the search names, in the order Oracle gives them.
const WANTED: [Color; 3] = [Color::Black, Color::Green, Color::Blue];

/// Which of the three wanted colors `card` is, as bits of `WANTED`: "a black card" and the others, their color words read as
/// the Dancers' text has them (CR 612).
fn slots_of(q: &dyn Query, source: ObjectId, card: ObjectId) -> u8 {
    let words = q.words(source);
    let colors = q.chars(card).colors;
    WANTED.iter().enumerate().fold(0, |bits, (slot, &color)| if colors.has(words.color(color)) { bits | 1 << slot } else { bits })
}

/// Whether each card, given by the wanted colors it is, can be taken as a different one of the three: "a black card, a green
/// card, and a blue card" are three separate cards (the ruling), though one card may be several of those colors.
fn one_to_one(slots: &[u8], taken: u8) -> bool {
    match slots.split_first() {
        None => true,
        Some((&mine, rest)) => (0..WANTED.len()).any(|slot| mine & !taken & (1 << slot) != 0 && one_to_one(rest, taken | 1 << slot)),
    }
}

/// "A black card, a green card, and a blue card" (CR 701.23): the cards chosen can each be one of those, a different one each
/// (the ruling). Any of the cards of such a set are such a set too.
fn black_green_and_blue(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let slots = |cards: &[ObjectId]| cards.iter().map(|&c| slots_of(q, choice.source, c)).collect::<Vec<u8>>();
    let addable = Bits::of(choice.candidates.len(), |i| {
        let with: Vec<ObjectId> = choice.selected.iter().copied().chain([choice.candidates[i]]).collect();
        !choice.selected.contains(&choice.candidates[i]) && one_to_one(&slots(&with), 0)
    });
    Verdict { complete: one_to_one(&slots(choice.selected), 0), addable }
}

const BLACK_GREEN_AND_BLUE: Part = select::part(&(black_green_and_blue as SelectFn), 0, "a black, a green and a blue card");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        // "Search your library for a black card, a green card, and a blue card. Put those cards into your graveyard, then shuffle."
        // (CR 701.23, 701.24) Each may be left unfound (CR 701.23b), so up to three are taken, one card for each color it is
        // matched to; a multicolored card fills any one of its colors' places and does not limit the others (the ruling).
        |g, item| {
            library::find_to_graveyard(g, item.controller, item.object, select::Selection::new(select::up_to(WANTED.len() as u32)).with(BLACK_GREEN_AND_BLUE))?;
            Ok(())
        },
        "When this creature enters, search your library for a black card, a green card, and a blue card. Put those cards into your graveyard, then shuffle.",
    )],
    ..oracle::LOTUSLIGHT_DANCERS
};
