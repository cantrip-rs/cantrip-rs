use cantrip_api::*;
use cantrip_mechanics::{power_up, tokens};
use cantrip_oracle as oracle;

/// The Tiger God, a legendary 4/4 green Cat God creature token with "The Tiger God can't be blocked by more than one creature."
/// (CR 111.4, 205.4a): named by the effect that makes it, so it has the name The Tiger God and no mana cost. Its blocking
/// restriction is its own ability (CR 509.1b, 113.1a), so a Tiger God that loses all abilities can be blocked by any number.
pub static THE_TIGER_GOD: CardDef = CardDef {
    name: "The Tiger God",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    subtypes: SubtypeSet::of(&[Subtype::Cat, Subtype::God]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::CantBeBlockedByMoreThanOne]),
    text: "The Tiger God can't be blocked by more than one creature.",
    ..CardDef::BLANK
};

/// "Put a +1/+1 counter on White Tiger and create The Tiger God ..." (CR 122.1, 111.10a): the counter goes on White Tiger if it is
/// still on the battlefield as the same object (CR 400.7); the token is made whether or not it is, for the ability's controller.
fn power_up_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    tokens::create(g, item.controller, &THE_TIGER_GOD, 1)?;
    Ok(())
}

/// "Power-up \u{2014} {5}{G}: ..." (CR 702.193): activated only once, and for {1}{G} less (her mana cost) the turn she entered.
pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{5}{G}"),
        power_up_effect,
        "Power-up \u{2014} {5}{G}: Put a +1/+1 counter on White Tiger and create The Tiger God, a legendary 4/4 green Cat God creature token with \"The Tiger God can't be blocked by more than one creature.\" (Activate each power-up ability only once. Reduce the cost by her mana cost if she entered this turn.)",
    )],
    ..oracle::WHITE_TIGER_AVA_AYALA
};

pub static EXTRA: &[&CardDef] = &[&THE_TIGER_GOD];
