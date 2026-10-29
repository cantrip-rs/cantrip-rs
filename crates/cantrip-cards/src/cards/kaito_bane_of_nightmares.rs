use cantrip_api::*;
use cantrip_mechanics::statics::{base_pt, plus_pt};
use cantrip_mechanics::{effects, ninjutsu::ninjutsu, surveil, targets};
use cantrip_oracle as oracle;

// --- "During your turn, as long as Kaito has one or more loyalty counters on him, he's a 3/4 Ninja creature and has
// --- hexproof" (CR 613): three effects, one per layer, each asking the same question afresh on every pass.

/// His controller's turn (CR 500.1), and a loyalty counter on him (CR 122.1).
fn ninja_form(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId) -> bool {
    let state = view.state();
    source == affected && state.turn.active == state.obj(source).controller && state.obj(source).counters(CounterKind::Loyalty) > 0
}

fn affects(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    ninja_form(view, source, affected)
}

/// Layer 4: a replacement of his card types, not an addition (CR 613.1d): he stops being a planeswalker, and the
/// subtype Kaito goes with it. He stays legendary: a supertype is not a card type.
fn become_a_ninja_creature(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.types = TypeSet::of(&[CardType::Creature]);
    chars.subtypes = SubtypeSet::of(&[q.words(source).subtype(Subtype::Ninja)]);
}

// --- The abilities ------------------------------------------------------------------------------------------------

fn emblem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

/// 0: "Surveil 2. Then draw a card for each opponent who lost life this turn" (CR 119.3): losing, not the net, so an
/// opponent who lost 1 and gained 2 still counts.
fn surveil_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    surveil(g, item.controller, item.object, 2)?;
    let bleeding = cantrip_mechanics::history::opponents_who_lost_life_this_turn(g, item.controller) as u32;
    if bleeding > 0 {
        g.draw(item.controller, bleeding);
    }
    Ok(())
}

// --- The emblem ---------------------------------------------------------------------------------------------------

/// "Ninjas you control get +1/+1" (layer 7c), from the command zone (CR 114.4): every Ninja creature its owner
/// controls, Kaito himself among them while his own static ability makes him one.
fn ninjas_you_control(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    view.state().obj(affected).controller == view.state().obj(source).controller
        && chars.is(CardType::Creature)
        && chars.has_subtype(view.words(source).subtype(Subtype::Ninja))
}

pub static EMBLEM: CardDef = CardDef {
    name: "Kaito, Bane of Nightmares emblem",
    has_cost: false,
    statics: &[static_ability(Layer::PtModify, ninjas_you_control, plus_pt::<1, 1>, "Ninjas you control get +1/+1.")],
    text: "Ninjas you control get +1/+1.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ninjutsu(
            mana("{1}{U}{B}"),
            "Ninjutsu {1}{U}{B} ({1}{U}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)",
        ),
        ability(loyalty(1), emblem, "+1: You get an emblem with \"Ninjas you control get +1/+1.\""),
        ability(loyalty(0), surveil_then_draw, "0: Surveil 2. Then draw a card for each opponent who lost life this turn."),
        ability(loyalty(-2), effects::tap_and_stun::<2>, "\u{2212}2: Tap target creature. Put two stun counters on it.")
            .targets(&[targets::CREATURE]),
    ],
    statics: &[
        static_ability(Layer::Type, affects, become_a_ninja_creature, "During your turn, as long as Kaito has one or more loyalty counters on him, he's a Ninja creature."),
        static_ability(Layer::Ability, affects, cantrip_mechanics::statics::have_hexproof, "During your turn, as long as Kaito has one or more loyalty counters on him, he has hexproof.").continues(),
        static_ability(Layer::PtSet, affects, base_pt::<3, 4>, "During your turn, as long as Kaito has one or more loyalty counters on him, he's a 3/4.").continues(),
    ],
    ..oracle::KAITO_BANE_OF_NIGHTMARES
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];
