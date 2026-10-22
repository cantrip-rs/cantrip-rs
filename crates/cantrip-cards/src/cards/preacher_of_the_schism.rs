use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Has the most life or is tied for most life": no player still in the game has more (CR 800.4a leaves out a player who has
/// left). Asked as the attack is declared (CR 508.1m); once the ability has triggered, life totals no longer matter.
fn has_most_life(q: &dyn Query, player: PlayerId) -> bool {
    let state = q.state();
    state.apnap().all(|other| state.player(other).life <= state.player(player).life)
}

/// "Whenever this creature attacks the player with the most life or tied for most life": it attacks a player, never a
/// planeswalker or battle (the ruling), and that player has the most life.
fn attacks_player_with_most_life(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender: Target::Player(p) } if *attacker == me.source && has_most_life(q, *p) => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever this creature attacks while you have the most life or are tied for most life": whatever it attacks.
fn attacks_while_you_have_most_life(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == me.source && has_most_life(q, me.controller) => Fired::PLAIN,
        _ => None,
    }
}

/// A 1/1 white Vampire creature token with lifelink.
static VAMPIRE: CardDef = CardDef {
    name: "Vampire Token",
    token_name: Some(&[Subtype::Vampire]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Vampire]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Lifelink]),
    ..CardDef::BLANK
};

/// "Create a 1/1 white Vampire creature token with lifelink."
fn create_vampire(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &VAMPIRE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ATTACKS, attacks_player_with_most_life),
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
