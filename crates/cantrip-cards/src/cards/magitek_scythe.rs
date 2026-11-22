use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::effects::{ask_may, gain_until_end_of_turn};
use cantrip_mechanics::lasting_rules::the_object_named;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "You may attach it to target creature you control. If you do, that creature gains first strike until end of turn and must be
/// blocked this turn if able." (CR 701.3a, 608.2d, 611.2a, 509.1c): the ability does nothing when its target is no longer legal
/// (CR 608.2b); the choice is made as it resolves; the creature gets both only if this Equipment became attached to it (CR 701.3b: not
/// one that has left the battlefield), and the requirement names that creature alone.
fn may_attach_then_first_strike_and_must_be_blocked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if !g.is_on_battlefield(item.object) {
        return Ok(());
    }
    if ask_may(g, item.controller, item.object, Purpose::Attach)? && g.attach(item.object, creature) {
        // "If you do" follows the instruction to attach (CR 608.2c): the state triggers judge the attaching first.
        g.end_sentence();
        // One sentence about one creature (CR 608.2c): the state triggers see both changes done together.
        return g.one_instruction(&mut |g| {
            gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
            g.add_lasting_rule(item, 0, Target::Object(creature), Duration::EndOfTurn);
            Ok(())
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            may_attach_then_first_strike_and_must_be_blocked,
            "A Test of Your Reflexes! \u{2014} When this Equipment enters, you may attach it to target creature you control. If you do, that creature gains first strike until end of turn and must be blocked this turn if able.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 1>, "Equipped creature gets +2/+1.")],
    lasting_rules: &[LastingRule {
        text: "That creature must be blocked this turn if able.",
        rule: LastingRuleKind::MustBeBlocked,
        applies: the_object_named,
    }],
    ..oracle::MAGITEK_SCYTHE
};
