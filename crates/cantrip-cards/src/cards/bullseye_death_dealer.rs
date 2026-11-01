use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "When Bullseye enters, you may sacrifice an artifact or discard a nonland card. When you do" (CR 603.12, 608.2c-d, 701.9,
/// 701.21): chosen as the triggered ability resolves, and the reflexive trigger is created only if the sacrifice or the
/// discard was actually done.
fn sacrifice_or_discard_for_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::may_sacrifice_an_artifact_or_discard(g, item.controller, item.object, select::NONLAND_CARD)? {
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(g.card_id(&CARD), 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            sacrifice_or_discard_for_damage,
            "When Bullseye enters, you may sacrifice an artifact or discard a nonland card. When you do, Bullseye deals 2 damage to any target.",
        ),
        ability(cost("{3}, {T}"), effects::damage_first_target::<2>, "{3}, {T}, Sacrifice an artifact or discard a nonland card: Bullseye deals 2 damage to any target.")
            .targets(&[targets::ANY])
            .pay_one_of(&[
                ExtraCost { text: "Sacrifice an artifact", cost: sacrifice(1, targets::filters::artifact) },
                ExtraCost { text: "Discard a nonland card", cost: discard(1, targets::filters::nonland) },
            ]),
    ],
    delayed: &[delayed_trigger(effects::damage_first_target::<2>, "When you do, Bullseye deals 2 damage to any target.").targets(&[targets::ANY])],
    labels: &["Sacrifice an artifact", "Discard a nonland card", "Neither"],
    ..oracle::BULLSEYE_DEATH_DEALER
};
