use cantrip_api::*;
use cantrip_mechanics::replacements::OPPONENTS_CREATURES_EXILED_INSTEAD_OF_DYING;
use cantrip_mechanics::triggers::{EACH_END_STEP, ward};
use cantrip_mechanics::statics::itself;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "This token gets +1/+1 for each other Rat you control" (layer 7c, CR 613.4c): one for each other Rat on the battlefield
/// under the token's controller, the Rat creature type read through the words of the token's own text (CR 612).
fn plus_one_for_each_other_rat(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let you = state.obj(source).controller;
    let rat = q.words(source).subtype(Subtype::Rat);
    let count = state.battlefield().iter().filter(|&&id| id != source && state.obj(id).controller == you && q.chars(id).has_subtype(rat)).count() as i16;
    chars.power = chars.power.map(|v| v + count);
    chars.toughness = chars.toughness.map(|v| v + count);
}

/// The 1/1 black Rat creature token with "This token gets +1/+1 for each other Rat you control." (CR 111.1).
pub static RAT: CardDef = CardDef {
    name: "Rat Token",
    token_name: Some(&[Subtype::Rat]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Rat]),
    power: Some(1),
    toughness: Some(1),
    statics: &[static_ability(Layer::PtModify, itself, plus_one_for_each_other_rat, "This token gets +1/+1 for each other Rat you control.")],
    text: "This token gets +1/+1 for each other Rat you control.",
    ..CardDef::BLANK
};

/// The creatures exiled under an opponent of `player`'s control this turn (CR 614.1a): every move this turn from the
/// battlefield to exile of an object that was a creature as it left (last known information, CR 608.2h) and that one of
/// `player`'s opponents controlled as it left. A token counts (it was exiled before it ceased to exist), and so does a
/// creature exiled by anything, not only by Vren's own replacement.
fn creatures_exiled_under_opponents_control_this_turn(q: &dyn Query, player: PlayerId) -> usize {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).to(ZoneKind::Exile).this_turn())
        .filter(|r| {
            matches!(r.event, Event::ZoneChange { old, .. }
                if q.state().last_known(old).is_some_and(|l| l.types.has(CardType::Creature))
                    && q.state().opponents(player).any(|o| o == q.obj(old).controller))
        })
        .count()
}

/// "At the beginning of each end step, create X 1/1 black Rat creature tokens ..., where X is the number of creatures that
/// were exiled under your opponents' control this turn." X is counted as the ability resolves; none is no tokens.
fn make_rats(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = creatures_exiled_under_opponents_control_this_turn(g, item.controller);
    tokens::create(g, item.controller, &RAT, x as u32)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(EACH_END_STEP, make_rats, "At the beginning of each end step, create X 1/1 black Rat creature tokens with \"This token gets +1/+1 for each other Rat you control,\" where X is the number of creatures that were exiled under your opponents' control this turn."),
    ],
    replacements: &[OPPONENTS_CREATURES_EXILED_INSTEAD_OF_DYING],
    ..oracle::VREN_THE_RELENTLESS
};

pub static EXTRA: &[&CardDef] = &[&RAT];
