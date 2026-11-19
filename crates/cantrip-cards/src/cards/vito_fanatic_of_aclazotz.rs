use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// A 4/3 white and black Vampire Demon creature token with flying (CR 111).
static VAMPIRE_DEMON: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::White).0 | ColorSet::of(Color::Black).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..token!(Vampire Demon)
};

/// "Whenever you sacrifice another permanent" (CR 701.21a): a permanent of yours other than this creature that a sacrifice moved
/// from the battlefield, read as it was when it left (CR 603.10a). One trigger for each permanent, however many are sacrificed
/// together; this creature sacrificed alongside them sees them (CR 603.10a), and does not trigger for itself.
fn you_sacrifice_another_permanent(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let sacrificed = triggers::sacrificed_by(q, me.controller, event)?;
    if sacrificed != me.source { Fired::PLAIN } else { None }
}

/// What the trigger does depends on how many times this ability of this Vito has resolved this turn before (CR 608.2m): this
/// turn's history counts them. First, gain 2 life. Second, each opponent loses 2 life. Third, create the 4/3 Vampire Demon.
/// A fourth and later resolution does nothing.
fn sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    match g.history().resolutions(item.object, item.ability) {
        0 => g.gain_life(item.controller, 2),
        1 => return effects::each_opponent_loses_2_life(g, item),
        2 => {
            tokens::create(g, item.controller, &VAMPIRE_DEMON, 1)?;
        }
        _ => {}
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, you_sacrifice_another_permanent),
        sacrifice,
        "Whenever you sacrifice another permanent, you gain 2 life if this is the first time this ability has resolved this turn. If it's the second time, each opponent loses 2 life. If it's the third time, create a 4/3 white and black Vampire Demon creature token with flying.",
    )
    .on_sacrifice()],
    ..oracle::VITO_FANATIC_OF_ACLAZOTZ
};

pub static EXTRA: &[&CardDef] = &[&VAMPIRE_DEMON];
