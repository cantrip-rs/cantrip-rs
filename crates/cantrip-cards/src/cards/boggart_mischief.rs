use cantrip_api::*;
use cantrip_mechanics::{blight, effects, tokens, triggers};
use cantrip_oracle as oracle;

const BLIGHT_ONE: Question = Question::new("BlightOne", "blight 1?", "", "Blight 1", "Don't");

/// The 1/1 black and red Goblin creature token (CR 111.1, 111.4).
pub static GOBLIN_1_1_BLACK_RED: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::Black).0 | ColorSet::of(Color::Red).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Goblin)
};

/// "You may blight 1. If you do, create two 1/1 black and red Goblin creature tokens" (CR 701.68a, 111.1).
fn blight_then_create_two_goblins(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if blight::may_in_effect(g, item.controller, item.object, 1, &BLIGHT_ONE)? {
        tokens::create(g, item.controller, &GOBLIN_1_1_BLACK_RED, 2)?;
    }
    Ok(())
}

/// "Whenever a Goblin creature you control dies" (CR 700.4, 603.10a): a creature the controller had on the battlefield, as it last
/// was there, so a changeling counts (CR 702.73a) and a token that died does too.
fn goblin_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::creature_you_control_dies(q, me, event)?;
    let Event::ZoneChange { old, .. } = event else { return None };
    q.chars(*old).has_subtype(q.words(me.source).subtype(Subtype::Goblin)).then_some(fired)
}

pub static EXTRA: &[&CardDef] = &[&GOBLIN_1_1_BLACK_RED];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            blight_then_create_two_goblins,
            "When this enchantment enters, you may blight 1. If you do, create two 1/1 black and red Goblin creature tokens.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, goblin_you_control_dies),
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever a Goblin creature you control dies, each opponent loses 1 life and you gain 1 life.",
        ),
    ],
    ..oracle::BOGGART_MISCHIEF
};
