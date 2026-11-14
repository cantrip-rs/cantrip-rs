use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// The X/X blue Spirit creature token with flying, its size given as it is created (CR 111.4, 707.2).
pub static SPIRIT: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet::of(Color::Blue)),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..token!(Spirit)
};

/// "When this creature dies, create an X/X blue Spirit creature token with flying, where X is the number of counters on this
/// creature" (CR 608.2h, 122.1, 111.4): counters of every kind, +1/+1 and -1/-1 included (its ruling 2024-09-20), as the creature
/// last existed on the battlefield. The token is X/X as it is made, so it never exists as a 0/0 first, and is made from this
/// card's text, so a text change reaches its type (CR 612.2a).
fn create_a_spirit_x_x_for_the_counters_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = effects::counters_on(g, item.object) as i32;
    let mut values = Copiable::of(g.card_id(&SPIRIT)).sized(x, x);
    values.words = g.words(item.object);
    g.create_token_copies_of(values, item.controller, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::EERIE,
            effects::counters_on_self::<{ CounterKind::Possession as usize }, 1>,
            "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, put a possession counter on this creature.",
        ),
        triggered(
            triggers::THIS_DIES,
            create_a_spirit_x_x_for_the_counters_on_this,
            "When this creature dies, create an X/X blue Spirit creature token with flying, where X is the number of counters on this creature.",
        ),
    ],
    ..oracle::UNWILLING_VESSEL
};

pub static EXTRA: &[&CardDef] = &[&SPIRIT];
