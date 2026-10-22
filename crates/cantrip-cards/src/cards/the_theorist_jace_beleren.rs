use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// "At the beginning of each opponent's draw step" (CR 503.1a, 603.2b): the draw step of a turn that is not its
/// controller's.
fn opponents_draw_step(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::Draw } if q.state().turn.active != q.controller(source) => Fired::PLAIN,
        _ => None,
    }
}

/// "You draw a card."
fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// The 1/1 blue Illusion creature token the +1 makes.
pub static ILLUSION: CardDef = CardDef {
    name: "Illusion Token",
    token_name: Some(&[Subtype::Illusion]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Illusion]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "+1: Create a 1/1 blue Illusion creature token."
fn make_illusion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create(g, item.controller, &ILLUSION, 1).map(|_| ())
}

/// The slot for one opponent of "for each opponent, up to one target artifact or creature that player controls" (CR 601.2c,
/// 115.1): an artifact or creature that opponent controls. The core makes a slot of its own for each opponent
/// (`TargetSpec::EACH_OPPONENT`) and says which one this is.
fn controlled_by_the_slots_opponent(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    targets::controlled_by_slot_opponent(q, id) && filters::artifact_or_creature(q, asker, id)
}

const FOR_EACH_OPPONENT: TargetSpec = target(controlled_by_the_slots_opponent).between(0, TargetSpec::EACH_OPPONENT).labelled("Return up to one target artifact or creature {player} controls to its owner's hand");

/// "\u{2212}2: For each opponent, return up to one target artifact or creature that player controls to its owner's
/// hand." The targets still legal are returned together (CR 608.2b).
fn bounce(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects = targets::legal_objects(g, item);
    g.return_to_hand(&objects);
    Ok(())
}

/// "\u{2212}6: Draw three cards. Then put X +1/+1 counters on each creature you control, where X is the number of cards
/// in your hand." X is counted after the draw.
fn ultimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.draw(you, 3);
    let x = g.state().zone(Zone::hand(you)).len() as i16;
    cantrip_mechanics::effects::counters_on_creatures_you_control(g, you, CounterKind::PlusOne, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when(EventMask::STEP_BEGAN, opponents_draw_step), draw_a_card, "At the beginning of each opponent's draw step, you draw a card."),
        ability(loyalty(1), make_illusion, "+1: Create a 1/1 blue Illusion creature token."),
        ability(loyalty(-2), bounce, "\u{2212}2: For each opponent, return up to one target artifact or creature that player controls to its owner's hand.")
            .targets(&[FOR_EACH_OPPONENT]),
        ability(loyalty(-6), ultimate, "\u{2212}6: Draw three cards. Then put X +1/+1 counters on each creature you control, where X is the number of cards in your hand."),
    ],
    ..oracle::THE_THEORIST_JACE_BELEREN
};

pub static EXTRA: &[&CardDef] = &[&ILLUSION];
