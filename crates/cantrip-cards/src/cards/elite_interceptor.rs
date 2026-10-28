use cantrip_api::*;
use cantrip_mechanics::{choose, effects, prepare, targets};
use cantrip_oracle as oracle;

/// "Rejoinder - You may tap or untap target creature. Draw a card." The draw is a second sentence (CR 608.2c): it happens
/// whether or not the creature was tapped or untapped. With its only target gone the spell does not resolve (CR 608.2b),
/// so it never draws.
fn rejoinder(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&REJOINDER);
    choose::may_tap_or_untap_target(g, item, card)?;
    g.end_sentence();
    effects::draw(g, item, 1)
}

/// Elite Interceptor's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static REJOINDER: CardDef = CardDef {
    name: "Rejoinder",
    cost: mana("{1}{W}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    labels: choose::TAP_OR_UNTAP,
    spell: spell(&[targets::CREATURE], rejoinder),
    text: "You may tap or untap target creature.\nDraw a card.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&REJOINDER),
    ..oracle::ELITE_INTERCEPTOR
};

pub static EXTRA: &[&CardDef] = &[&REJOINDER];
