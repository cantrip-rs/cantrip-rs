use cantrip_api::*;
use cantrip_mechanics::select::{self, Selection};
use cantrip_mechanics::statics::{has_delayed_ability, your_creatures};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, exile_and_return_transformed, lands, library, tokens};
use cantrip_oracle as oracle;

/// "When Huatli enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle." (CR 701.23):
/// the search may find nothing, and the library is shuffled either way.
fn fetch_a_basic(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_hand(g, item.controller, item.object)
}

/// "Exile Huatli, then return her to the battlefield transformed under her owner's control" (CR 712.14a): the owner's, whoever
/// controlled Huatli; nothing if she has left the battlefield already, since the new object is not the one the ability means
/// (CR 400.7).
fn transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, fetch_a_basic, "When Huatli enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle."),
        ability(cost("{3}{R/W}{R/W}"), transform, "{3}{R/W}{R/W}: Exile Huatli, then return her to the battlefield transformed under her owner's control. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::HUATLI_POET_OF_UNITY
};

/// I: "Create two 3/3 green Dinosaur creature tokens."
fn make_dinosaurs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::GREEN_DINOSAUR, 2)?;
    Ok(())
}

/// II: "This Saga gains 'Creatures you control have "{T}: Add {R}, {G}, or {W}."'" (CR 611.2a, 613.1f): the static ability is given
/// to the object that is the Saga now, with no duration, so it lasts while the Saga stays (a Saga that has left the battlefield is
/// another object, CR 400.7, and is given nothing). The static ability in turn gives each creature the mana ability.
fn gain_the_static(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::Indefinite);
    Ok(())
}

const DINOSAUR_CARD: Selection = Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Dinosaur)]).called("Dinosaur");

/// III: "Search your library for a Dinosaur card, reveal it, put it into your hand, then shuffle." (CR 701.23): "Dinosaur" is a
/// word of this card's text (CR 612.2), and the search may find nothing (CR 701.23b).
fn find_a_dinosaur(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, DINOSAUR_CARD, true)?;
    Ok(())
}

/// IV: "Dinosaurs you control gain double strike and trample until end of turn." (CR 611.2c): the Dinosaurs there are as it
/// resolves, and none that enters later (`effects::of_subtype_you_control_gain_until_end_of_turn`).
const DINOSAURS_GAIN_DOUBLE_STRIKE_AND_TRAMPLE: fn(&mut dyn Game, &StackItem) -> Result<(), Suspend> =
    effects::of_subtype_you_control_gain_until_end_of_turn::<{ lands::sub(Subtype::Dinosaur) }, { Keywords::of(&[Keyword::DoubleStrike, Keyword::Trample]).0 }>;

/// Roar of the Fifth People, the Saga side (CR 714). Its delayed ability 0 is the static ability chapter II gives it, which gives
/// every creature its controller has delayed ability 1, the mana ability (CR 113.7: the creature itself is its source).
pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, make_dinosaurs, "I \u{2014} Create two 3/3 green Dinosaur creature tokens."),
        chapter(2, gain_the_static, "II \u{2014} This Saga gains \"Creatures you control have '{T}: Add {R}, {G}, or {W}.'\""),
        chapter(3, find_a_dinosaur, "III \u{2014} Search your library for a Dinosaur card, reveal it, put it into your hand, then shuffle."),
        chapter(4, DINOSAURS_GAIN_DOUBLE_STRIKE_AND_TRAMPLE, "IV \u{2014} Dinosaurs you control gain double strike and trample until end of turn."),
    ],
    delayed: &[
        granted_static_ability(Layer::Ability, your_creatures, has_delayed_ability::<1>, "Creatures you control have \"{T}: Add {R}, {G}, or {W}.\""),
        mana_ability(TAP, add_one_of(&[Color::Red, Color::Green, Color::White]), "{T}: Add {R}, {G}, or {W}."),
    ],
    ..oracle::ROAR_OF_THE_FIFTH_PEOPLE
};

pub static EXTRA: &[&CardDef] = &[&tokens::GREEN_DINOSAUR];
