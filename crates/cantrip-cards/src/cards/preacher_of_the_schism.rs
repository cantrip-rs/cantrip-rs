use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks while you have the most life or are tied for most life": whatever it attacks.
fn attacks_while_you_have_most_life(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == me.source && triggers::has_most_life(q, me.controller) => Fired::PLAIN,
        _ => None,
    }
}

/// A 1/1 white Vampire creature token with lifelink.
static VAMPIRE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Lifelink]),
    ..token!(Vampire)
};

/// "Create a 1/1 white Vampire creature token with lifelink."
fn create_vampire(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &VAMPIRE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ATTACKS, triggers::this_attacks_the_player_with_the_most_life),
            create_vampire,
            "Whenever this creature attacks the player with the most life or tied for most life, create a 1/1 white Vampire creature token with lifelink.",
        ),
        triggered(
            when_asked(EventMask::ATTACKS, attacks_while_you_have_most_life),
            effects::draw_a_card_and_lose_1_life,
            "Whenever this creature attacks while you have the most life or are tied for most life, you draw a card and you lose 1 life.",
        ),
    ],
    ..oracle::PREACHER_OF_THE_SCHISM
};

pub static EXTRA: &[&CardDef] = &[&VAMPIRE];
