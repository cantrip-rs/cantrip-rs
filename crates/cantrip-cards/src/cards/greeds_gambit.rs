use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, THIS_LEAVES, YOUR_END_STEP};
use cantrip_mechanics::{discard_chosen, sacrifice_chosen, tokens};
use cantrip_oracle as oracle;

/// A 2/1 black Bat creature token with flying.
pub static BAT: CardDef = CardDef {
    name: "Bat Token",
    token_name: Some(&[Subtype::Bat]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Bat]),
    power: Some(2),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

fn is_creature(chars: &Chars) -> bool {
    chars.is(CardType::Creature)
}

/// "You draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying", in that order.
fn gambit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
    g.gain_life(item.controller, 6);
    tokens::create(g, item.controller, &BAT, 3)?;
    Ok(())
}

/// "You discard a card, lose 2 life, and sacrifice a creature", in that order: the discard is chosen when there is a card to
/// discard (CR 701.9), the life lost whatever else could not be done, and the creature chosen when there is a choice
/// (CR 701.21), one of those the discard and the life loss left, with the Bats among them.
fn pay_the_price(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discard_chosen(g, item.controller, item.object, 1)?;
    g.lose_life(item.controller, 2);
    sacrifice_chosen(g, item.controller, item.object, 1, is_creature).map(drop)
}

/// "You discard three cards, lose 6 life, and sacrifice three creatures", in that order. Fewer cards or creatures than that
/// are all discarded or sacrificed; the life is lost all the same.
fn pay_the_bill(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discard_chosen(g, item.controller, item.object, 3)?;
    g.lose_life(item.controller, 6);
    sacrifice_chosen(g, item.controller, item.object, 3, is_creature).map(drop)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, gambit, "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying."),
        triggered(YOUR_END_STEP, pay_the_price, "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature."),
        triggered(THIS_LEAVES, pay_the_bill, "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures."),
    ],
    ..oracle::GREEDS_GAMBIT
};

pub static EXTRA: &[&CardDef] = &[&BAT];
