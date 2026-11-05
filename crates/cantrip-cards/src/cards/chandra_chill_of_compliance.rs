use cantrip_api::*;
use cantrip_mechanics::{surveil_into_graveyard, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// +1: "Surveil 1. If you put a noncreature, nonland card into your graveyard this way, put that card into your hand."
/// The card is the one the surveil really put into the graveyard (CR 701.25a), judged there as it is now.
fn surveil_and_take(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let binned = surveil_into_graveyard(g, item.controller, item.object, 1)?;
    let taken: Vec<ObjectId> = binned
        .into_iter()
        .filter(|&card| {
            let chars = g.chars(card);
            !chars.is(CardType::Creature) && !chars.is(CardType::Land)
        })
        .collect();
    g.return_to_hand(&taken);
    Ok(())
}

/// +1: "Add {U}. Spend this mana only to cast a noncreature spell." A loyalty ability, so no mana ability (CR 605.1a): it
/// resolves from the stack, at sorcery speed. The mana pays only for casting a spell that isn't a creature spell (an
/// artifact creature spell is one), and nothing that isn't a spell (CR 106.6).
fn blue_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_restricted_mana(item.controller, Mana::U, 1, ManaRestriction::NONCREATURE_SPELLS);
    Ok(())
}

/// −X: "Tap target artifact or creature. Put X stun counters on it." X is announced as the ability is activated, up to
/// Chandra's loyalty (CR 606.4, 107.3); X of 0 still taps it. Its one target illegal, the ability does not resolve at
/// all (CR 608.2b).
fn tap_and_stun(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(permanent);
    if item.x > 0 {
        g.add_counters(permanent, CounterKind::Stun, i64::from(item.x as i16));
    }
    Ok(())
}

/// −6: "You get an emblem with ..." (CR 114.1).
fn emblem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

/// The emblem's "Whenever you cast a spell, draw a card." (CR 114.4: it functions from the command zone.)
fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static EMBLEM: CardDef = CardDef {
    name: "Chandra, Chill of Compliance emblem",
    has_cost: false,
    abilities: &[triggered(triggers::YOU_CAST_A_SPELL, draw, "Whenever you cast a spell, draw a card.")],
    text: "Whenever you cast a spell, draw a card.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), surveil_and_take, "+1: Surveil 1. If you put a noncreature, nonland card into your graveyard this way, put that card into your hand."),
        ability(loyalty(1), blue_mana, "+1: Add {U}. Spend this mana only to cast a noncreature spell."),
        ability(loyalty_minus_x(), tap_and_stun, "\u{2212}X: Tap target artifact or creature. Put X stun counters on it.")
            .targets(&[targets::ARTIFACT_OR_CREATURE]),
        ability(loyalty(-6), emblem, "\u{2212}6: You get an emblem with \"Whenever you cast a spell, draw a card.\""),
    ],
    ..oracle::CHANDRA_CHILL_OF_COMPLIANCE
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];
