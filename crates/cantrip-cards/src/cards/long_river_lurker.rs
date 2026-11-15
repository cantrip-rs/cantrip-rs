use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, THIS_DEALS_COMBAT_DAMAGE, THIS_ENTERS};
use cantrip_mechanics::{effects, lands, statics, targets};
use cantrip_oracle as oracle;

const WARD: AbilityDef = ward::<1>("Ward {1} (Whenever this permanent becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)");

/// "When this creature enters, target creature you control can't be blocked this turn. Whenever that creature deals combat damage
/// this turn, you may exile it. If you do, return it to the battlefield under its owner's control." (CR 509.1b, 603.7): the
/// unblockable effect, then a delayed trigger on that creature for the rest of the turn (the card's second delayed ability),
/// once for each combat damage step it deals damage in (CR 510.2: its damage is one batch).
fn unblockable_and_watch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::target_creature_cant_be_blocked_this_turn(g, item)?;
    g.delayed_trigger(DelayedWhen::ThisTurn, creature, item.controller, AbilityRef::delayed(item.ability.card, 1), Target::None);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        WARD,
        triggered(
            THIS_ENTERS,
            unblockable_and_watch,
            "When this creature enters, target creature you control can't be blocked this turn. Whenever that creature deals combat damage this turn, you may exile it. If you do, return it to the battlefield under its owner's control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    delayed: &[
        WARD,
        triggered(
            THIS_DEALS_COMBAT_DAMAGE,
            |g, item| effects::may_exile_then_return(g, item, item.object),
            "Whenever that creature deals combat damage this turn, you may exile it. If you do, return it to the battlefield under its owner's control.",
        )
        .once_per_batch(),
    ],
    statics: &[static_ability(
        Layer::Ability,
        statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Frog) }>,
        statics::grant_delayed_ability,
        "Other Frogs you control have ward {1}.",
    )],
    ..oracle::LONG_RIVER_LURKER
};
