use cantrip_api::*;
use cantrip_mechanics::chosen_type::{creature_type_now, creatures_you_control_of_chosen_type};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{effects, library, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your first main phase, you may pay {G}. If you do, transform Oko."
fn pay_green(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{G}"))
}

/// "At the beginning of your first main phase, you may pay {U}. If you do, transform Oko."
fn pay_blue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{U}"))
}

/// "+2: Up to one target creature gains all creature types. (This effect doesn't end.)" Not Changeling: it is the types it
/// gains (CR 205.3d, 613.1d), for good (CR 611.2a). Nothing with no target chosen or one no longer legal (CR 608.2b).
fn gains_all_creature_types(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::AddAllCreatureTypes, Duration::Indefinite);
    Ok(())
}

/// "+1: Target creature gets -2/-0 until your next turn." It lasts as long as stated (CR 611.2a): through the rest of this
/// turn and the opponents' turns, ending as the ability's controller's next turn begins.
fn weakens_until_your_next_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::Pump(-2, 0), Duration::UntilNextTurnOf(item.controller));
    Ok(())
}

/// "-1: Mill three cards. You may put a permanent card from among them into your hand."
fn mill_three_and_take_a_permanent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_may_take_into_hand(g, item.controller, item.object, 3, "permanent", targets::is_permanent_card)?;
    Ok(())
}

/// The 3/3 green Elk creature token.
pub static ELK: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..token!(Elk)
};

/// "-3: Create two 3/3 green Elk creature tokens."
fn two_elks(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ELK, 2)?;
    Ok(())
}

/// "-6: Choose a creature type. You get an emblem with ..." The type is chosen as the ability resolves (CR 608.2d) and kept on
/// the emblem, which every ability of it reads (CR 607.2d).
fn emblem_of_a_creature_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let chosen = creature_type_now(g, item.controller, item.object)?;
    tokens::create_emblem_choosing_subtype(g, item.controller, &EMBLEM, chosen);
    Ok(())
}

fn vigilance_and_hexproof(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = Keywords(chars.keywords.0 | Keywords::of(&[Keyword::Vigilance, Keyword::Hexproof]).0);
}

/// The emblem: its statics apply continuously (CR 113.6, 611.3a), so a creature that becomes the chosen type later is
/// affected, and one that stops being it is not.
pub static EMBLEM: CardDef = CardDef {
    name: "Oko, Shadowmoor Scion emblem",
    has_cost: false,
    statics: &[
        static_ability(Layer::PtModify, creatures_you_control_of_chosen_type, plus_pt::<3, 3>, "Creatures you control of the chosen type get +3/+3 and have vigilance and hexproof."),
        static_ability(Layer::Ability, creatures_you_control_of_chosen_type, vigilance_and_hexproof, "Creatures you control of the chosen type get +3/+3 and have vigilance and hexproof."),
    ],
    text: "Creatures you control of the chosen type get +3/+3 and have vigilance and hexproof.",
    ..CardDef::BLANK
};

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_blue, "At the beginning of your first main phase, you may pay {U}. If you do, transform Oko."),
        ability(loyalty(-1), mill_three_and_take_a_permanent, "\u{2212}1: Mill three cards. You may put a permanent card from among them into your hand."),
        ability(loyalty(-3), two_elks, "\u{2212}3: Create two 3/3 green Elk creature tokens."),
        ability(loyalty(-6), emblem_of_a_creature_type, "\u{2212}6: Choose a creature type. You get an emblem with \"Creatures you control of the chosen type get +3/+3 and have vigilance and hexproof.\""),
    ],
    ..oracle::OKO_SHADOWMOOR_SCION
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_green, "At the beginning of your first main phase, you may pay {G}. If you do, transform Oko."),
        ability(loyalty(2), gains_all_creature_types, "+2: Up to one target creature gains all creature types. (This effect doesn't end.)").targets(&[targets::CREATURE.between(0, 1)]),
        ability(loyalty(1), weakens_until_your_next_turn, "+1: Target creature gets -2/-0 until your next turn.").targets(&[targets::CREATURE]),
    ],
    back: Some(&BACK),
    ..oracle::OKO_LORWYN_LIEGE
};

pub static EXTRA: &[&CardDef] = &[&ELK, &EMBLEM];
