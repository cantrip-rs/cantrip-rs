use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "If a source you control would deal damage to an opponent" (CR 615.1, 120.3): damage of any source its controller controls,
/// combat or not, of an amount above zero (damage already prevented to nothing is no damage to prevent, so a second one of this
/// card has none, the ruling), dealt to an opponent of the controller - not to a permanent an opponent controls.
fn damage_you_deal_to_an_opponent(q: &dyn Query, asker: &Asker, d: &DamageEvent) -> bool {
    matches!(d.target, Target::Player(_)) && replacements::damage_you_deal_to_an_opponent_or_their_permanent(q, asker, &Proposal::Damage(*d))
}

/// "... prevent that damage and each opponent mills that many cards" (CR 615.5, 701.17a, 101.4): every opponent of the
/// controller mills as many cards as the damage was, together (one instruction). The damage is read as the effect met it, as
/// the effects applied before it have left it (CR 616.1f). Damage that can't be prevented is still dealt, but this still
/// takes place (CR 615.12).
fn each_opponent_mills_that_many(g: &mut dyn Game, asker: &Asker, before: &Proposal, _: &Proposal) {
    let Proposal::Damage(d) = before else { return };
    let mills: Vec<(PlayerId, u32)> = g.state().opponents(asker.controller).map(|p| (p, d.amount)).collect();
    g.mill_together(&mills);
}

pub static CARD: CardDef = CardDef {
    preventions: &[PreventionDef {
        hint: "prevents the damage, and each opponent mills that many cards",
        applies: damage_you_deal_to_an_opponent,
        amount: PreventionAmount::All,
        then: Some(each_opponent_mills_that_many),
    }],
    ..oracle::THE_MINDSKINNER
};
