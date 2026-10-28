use cantrip_api::*;
use cantrip_mechanics::triggers::LANDFALL;
use cantrip_mechanics::{effects, lands, pump, tokens};
use cantrip_oracle as oracle;

/// I: "Search your library for a basic land card, reveal it, put it into your hand, then shuffle." (CR 701.23): not optional,
/// though the player may find nothing, and the library is shuffled either way.
fn fetch_a_basic(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_hand(g, item.controller, item.object)
}

/// II: "This Saga gains 'Landfall \u{2014} Whenever a land you control enters, create a 1/1 green Elf creature token.'" (CR 611.2a,
/// 613.1f): the ability is given to the object that is the Saga now, with no duration, so it lasts while the Saga stays (a
/// Saga that has left the battlefield is another object, CR 400.7, and is given nothing).
fn gain_landfall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = g.card_id(&CARD);
    g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(me, 0)), Duration::Indefinite);
    Ok(())
}

/// III, IV: "Elves you control get +1/+0 and gain vigilance until end of turn." (CR 611.2a, 611.2c): the Elves there are as it
/// resolves, and none that enters later; "Elf" is a word of this card's text (CR 612), and a permanent is one as the layers
/// make it (a changeling is every creature type).
fn elves_get_power_and_vigilance(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let elf = g.words(item.object).subtype(Subtype::Elf);
    let elves: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(elf)).collect();
    instruction(g, |g| {
        for id in elves {
            pump(g, item.object, id, 1, 0);
            effects::gain_until_end_of_turn(g, item.object, id, Keyword::Vigilance);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, fetch_a_basic, "I \u{2014} Search your library for a basic land card, reveal it, put it into your hand, then shuffle."),
        chapter(2, gain_landfall, "II \u{2014} This Saga gains \"Landfall \u{2014} Whenever a land you control enters, create a 1/1 green Elf creature token.\""),
        chapter(3, elves_get_power_and_vigilance, "III, IV \u{2014} Elves you control get +1/+0 and gain vigilance until end of turn."),
        chapter(4, elves_get_power_and_vigilance, "III, IV \u{2014} Elves you control get +1/+0 and gain vigilance until end of turn."),
    ],
    delayed: &[triggered(LANDFALL, tokens::create_an_elf, "Landfall \u{2014} Whenever a land you control enters, create a 1/1 green Elf creature token.")],
    ..oracle::DOWN_IN_THE_VALLEY
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELF];
