use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "X is twice the mana value of that spell" (CR 107.3c): X is defined by the text, so it is not announced; it is read as
/// the total cost is determined, once the target spell is announced (CR 601.2f by way of 602.2b), from that spell's mana
/// value on the stack, its own X counted (CR 202.3e).
fn twice_its_mana_value(q: &dyn Query, _: &Asker, targets: &[Target]) -> u32 {
    targets.first().and_then(|t| t.object()).map_or(0, |spell| 2 * q.mana_value(spell))
}

/// "The new target must be a player."
fn a_player(_: &dyn Query, t: Target) -> bool {
    matches!(t, Target::Player(_))
}

/// "Change the target of target spell with a single target if that target is you. The new target must be a player."
/// (CR 115.7a): only a spell whose one target is the ability's controller, as it resolves, has it changed, by that player, to
/// another player who is a legal target for it; with none, it stays (CR 115.7a). A spell that merely affects "you" without
/// targeting is no spell whose target is you (CR 115.10b, ruling 2004-10-04).
fn reflect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let you = Target::Player(item.controller);
    let targets_you = g.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == spell && g.state().targets(i).contains(&you));
    if !targets_you {
        return Ok(());
    }
    g.change_target_where(Target::Object(spell), item.controller, a_player)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{X}").and(TAP), reflect, "{X}, {T}: Change the target of target spell with a single target if that target is you. The new target must be a player. X is twice the mana value of that spell.")
        .targets(&[targets::SPELL_WITH_A_SINGLE_TARGET])
        .x_is(twice_its_mana_value)],
    ..oracle::REFLECTING_MIRROR
};
