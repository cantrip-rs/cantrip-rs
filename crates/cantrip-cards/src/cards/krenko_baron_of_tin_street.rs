use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_each_of_subtype_you_control;
use cantrip_mechanics::{haste_until_end_of_turn, lands, targets, tokens};
use cantrip_oracle as oracle;

pub use cantrip_mechanics::tokens::GOBLIN;

const PAY: Question = Question::new("Pay", "pay {R} to create a Goblin?", "", "Pay {R}", "Don't");

/// "Whenever an artifact is put into a graveyard from the battlefield" (CR 603.10a): any artifact, anyone's, a token too (it is put
/// into the graveyard before it ceases to exist, CR 704.5d), as it last was on the battlefield. A card that goes to a graveyard from
/// anywhere else, or a permanent that is not an artifact or goes to another zone, does not trigger it.
fn artifact_is_put_into_a_graveyard_from_the_battlefield(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) && q.chars(*old).is(CardType::Artifact) =>
        {
            Fired::PLAIN
        },
        _ => None,
    }
}

/// "You may pay {R}. If you do, create a 1/1 red Goblin creature token. It gains haste until end of turn." (CR 118.12, 608.2c-d,
/// 702.10): the cost is paid as the ability resolves, the token is created only if it was, and the token alone gets haste.
fn pay_for_a_hasty_goblin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{R}"), &PAY)? {
        g.end_sentence();
        let made = tokens::create(g, item.controller, &GOBLIN, 1)?;
        g.end_sentence();
        instruction(g, |g| {
            for &token in &made {
                haste_until_end_of_turn(g, item.object, token);
            }
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP.and(sacrifice(1, targets::filters::artifact)),
            plus_one_counter_on_each_of_subtype_you_control::<{ lands::sub(Subtype::Goblin) }>,
            "{T}, Sacrifice an artifact: Put a +1/+1 counter on each Goblin you control.",
        ),
        triggered(
            when(EventMask::ZONE_CHANGE, artifact_is_put_into_a_graveyard_from_the_battlefield),
            pay_for_a_hasty_goblin,
            "Whenever an artifact is put into a graveyard from the battlefield, you may pay {R}. If you do, create a 1/1 red Goblin creature token. It gains haste until end of turn.",
        ),
    ],
    ..oracle::KRENKO_BARON_OF_TIN_STREET
};

pub static EXTRA: &[&CardDef] = &[&GOBLIN];
