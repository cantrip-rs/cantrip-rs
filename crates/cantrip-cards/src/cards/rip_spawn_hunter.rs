use cantrip_api::*;
use cantrip_mechanics::library::reveal_top_select_to_hand_rest_to_bottom_in_random_order;
use cantrip_mechanics::select::{self, part};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Creature and/or Vehicle cards" (CR 205.2, 205.3, 612.2): a card that is a creature card or has the subtype Vehicle, as it is in
/// the library, the subtype read through the words of Rip's text.
fn creature_or_vehicle(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let vehicle = q.words(choice.source).subtype(Subtype::Vehicle);
    Verdict::each(choice, |card| q.chars(card).is(CardType::Creature) || q.chars(card).has_subtype(vehicle))
}

/// "With different powers" (CR 208.1): no two of the cards chosen have the same power, each read as it is in the library.
fn different_powers(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let power = |card: ObjectId| q.chars(card).power;
    let distinct = |cards: &[ObjectId]| cards.iter().enumerate().all(|(i, &a)| cards[..i].iter().all(|&b| power(a) != power(b)));
    let addable = Bits::of(choice.candidates.len(), |i| {
        let with: Vec<ObjectId> = choice.selected.iter().copied().chain([choice.candidates[i]]).collect();
        !choice.selected.contains(&choice.candidates[i]) && distinct(&with)
    });
    Verdict { complete: distinct(choice.selected), addable }
}

/// "Reveal the top X cards of your library, where X is its power. Put any number of creature and/or Vehicle cards with different
/// powers from among them into your hand. Put the rest on the bottom of your library in a random order." X is Rip's power as the
/// ability resolves (CR 608.2h, the ruling): as it last existed on the battlefield if it left, which `chars` reads.
fn reveal_its_power_in_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = usize::try_from(g.chars(item.object).power.unwrap_or(0).max(0)).unwrap_or(0);
    let rule = select::cards(select::ANY_NUMBER)
        .with(part(&(creature_or_vehicle as SelectFn), 0, ""))
        .with(part(&(different_powers as SelectFn), 0, "with different powers"))
        .called("creature or Vehicle");
    reveal_top_select_to_hand_rest_to_bottom_in_random_order(g, item.controller, item.object, x, rule)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::survival(
        reveal_its_power_in_cards,
        "Survival \u{2014} At the beginning of your second main phase, if Rip is tapped, reveal the top X cards of your library, where X is its power. Put any number of creature and/or Vehicle cards with different powers from among them into your hand. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::RIP_SPAWN_HUNTER
};
