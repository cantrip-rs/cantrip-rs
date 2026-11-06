use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "Prevent all noncombat damage that would be dealt to other creatures you control" (CR 615.1, 604.1): a prevention effect
/// of this static ability, while it is on the battlefield, of damage that is not combat damage and is dealt to a creature
/// its controller controls other than itself. Damage to Crystal Barricade itself, to its controller and to others'
/// creatures is not prevented.
fn noncombat_to_your_other_creatures(q: &dyn Query, asker: &Asker, d: &DamageEvent) -> bool {
    !d.combat
        && matches!(d.target, Target::Object(id)
            if id != asker.source && q.is_on_battlefield(id) && q.controller(id) == asker.controller && q.chars(id).is(CardType::Creature))
}

/// "Defender" is a core keyword. "You have hexproof" (CR 702.11c) is a player keyword ability of the permanent
/// (`statics::YOU_HAVE_HEXPROOF`).
pub static CARD: CardDef = CardDef {
    player_keywords: &[statics::YOU_HAVE_HEXPROOF],
    preventions: &[PreventionDef {
        hint: "prevents all noncombat damage that would be dealt to other creatures you control",
        applies: noncombat_to_your_other_creatures,
        amount: PreventionAmount::All,
    }],
    ..oracle::CRYSTAL_BARRICADE
};
