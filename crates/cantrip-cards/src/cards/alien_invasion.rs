use cantrip_api::*;
use cantrip_mechanics::statics::{attacks_each_combat_if_able, itself};
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// A 1/1 red Alien creature token with haste and "This token attacks each combat if able" (CR 111.4, 205.3m).
pub static ALIEN: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Haste]),
    statics: &[static_ability(Layer::Ability, itself, attacks_each_combat_if_able, "This token attacks each combat if able.")],
    ..token!(Alien)
};

/// "Create a [token]. Put a +1/+1 counter on it for each invasion counter on this enchantment, then put an invasion counter on this
/// enchantment": the count is read as the token is made, so the counter this resolution adds is not counted; the enchantment's
/// last-known counters stand if it has gone (CR 608.2h), and no counter is put on a permanent that is gone (CR 400.7).
fn invade(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let invasion = g.obj(item.object).counters(CounterKind::Invasion);
    let made = tokens::create(g, item.controller, &ALIEN, 1)?;
    for &alien in &made {
        g.add_counters(alien, CounterKind::PlusOne, invasion);
    }
    g.add_counters(item.object, CounterKind::Invasion, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        invade,
        "At the beginning of combat on your turn, create a 1/1 red Alien creature token with haste and \"This token attacks each combat if able.\" Put a +1/+1 counter on it for each invasion counter on this enchantment, then put an invasion counter on this enchantment.",
    )],
    ..oracle::ALIEN_INVASION
};

pub static EXTRA: &[&CardDef] = &[&ALIEN];
