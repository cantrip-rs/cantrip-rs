use cantrip_api::*;
use cantrip_oracle as oracle;

/// Whether `target` is a player or a permanent that `ok` says of the player it is or of the permanent's controller.
fn dealt_to(q: &dyn Query, target: Target, ok: impl Fn(PlayerId) -> bool) -> bool {
    target.object().map_or(target.player(), |id| Some(q.controller(id))).is_some_and(ok)
}

/// "If a source would deal damage to an opponent or a permanent an opponent controls" (CR 614.1a, 120.3): any source's
/// damage, an opponent's own included (the ruling), once it has been divided or assigned among its recipients (the ruling:
/// a trampler's 2 and 3 become 4 and 6). Damage already prevented to nothing is no damage (CR 615.6).
fn to_an_opponent(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.amount > 0 && dealt_to(q, d.target, |p| q.state().opponents(asker.controller).any(|o| o == p)))
}

/// "... that source deals double that damage to that player or permanent instead": still dealt by the source. The player
/// or controller of the permanent orders it with other replacement and prevention effects (CR 616.1, the ruling).
fn double_that_damage(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount = d.amount * 2;
    }
}

/// "If a source would deal damage to you or a permanent you control" (CR 615.1): a prevention effect of this static
/// ability, while it is on the battlefield (CR 604.1).
fn to_you(q: &dyn Query, asker: &Asker, d: &DamageEvent) -> bool {
    dealt_to(q, d.target, |p| p == asker.controller)
}

/// "... prevent half that damage, rounded up": of the damage as the effects applied before it have left it (CR 616.1f), so
/// 3 damage has 2 of it prevented and 1 dealt, and 1 damage is all prevented.
fn half_rounded_up(_: &dyn Query, _: &Asker, d: &DamageEvent) -> u32 {
    d.amount.div_ceil(2)
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::Damage,
        to_an_opponent,
        double_that_damage,
        "deals double that damage to an opponent or their permanent instead",
    )],
    preventions: &[PreventionDef { hint: "prevents half that damage, rounded up", applies: to_you, amount: PreventionAmount::Relative(half_rounded_up) }],
    ..oracle::GISELA_BLADE_OF_GOLDNIGHT
};
