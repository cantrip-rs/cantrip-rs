use cantrip_api::*;
use cantrip_mechanics::effects::two_plus_one_counters_on_this;
use cantrip_oracle as oracle;

/// "Whenever one or more Heroes you control deal damage to a player" (CR 120.2, 603.2c): damage of any kind dealt to a player by a
/// source that, as it dealt it, was a Hero under this permanent's controller's control (CR 120.3, 609.7) - Hero being a creature
/// type of this card's text (CR 612) - so a Hero that has since left or changed still counts. One trigger for each player the batch
/// damaged, however many Heroes dealt it (`sums_per_subject`).
fn heroes_you_control_deal_damage_to_a_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Damage { source, target: Target::Player(player), amount, kind, controller, known, .. } = *event else { return None };
    if amount == 0 || controller != me.controller { return None; }
    let dealt = cantrip_api::history::DamageDealt { source, target: Target::Player(player), amount, combat: kind.is_combat(), controller, known };
    let hero = q.words(me.source).subtype(Subtype::Hero);
    if dealt.chars(q).has_subtype(hero) { Fired::about(Target::Player(player)) } else { None }
}

/// Trample is a core keyword. "... put two +1/+1 counters on The Thing." (CR 122.1)
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, heroes_you_control_deal_damage_to_a_player),
        two_plus_one_counters_on_this,
        "Whenever one or more Heroes you control deal damage to a player, put two +1/+1 counters on The Thing.",
    )
    .sums_per_subject()],
    ..oracle::THE_THING_BEN_GRIMM
};
