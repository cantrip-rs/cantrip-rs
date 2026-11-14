use cantrip_api::*;
use cantrip_mechanics::{effects, mana, triggers};
use cantrip_oracle as oracle;

/// The Islands named by this creature's rules text (CR 612.1).
fn island(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Island))
}

/// You may sacrifice two Islands. If you do, untap this creature (CR 608.2c, 701.21, 701.26).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_untap_self(g, item, sacrifice(2, island))
}

/// The attack cost applies only to this creature (CR 508.1h).
fn this_attacks(_: &dyn Query, asker: &Asker, attacker: ObjectId, _: Target) -> bool {
    attacker == asker.source
}

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::Trample, Keyword::DoesNotUntap]),
    abilities: &[
        triggered(triggers::YOUR_UPKEEP, upkeep, "At the beginning of your upkeep, you may sacrifice two Islands. If you do, untap this creature."),
    ],
    enters: Some(mana::enters_tapped),
    attack_cost_rules: &[
        AttackPayment { applies: this_attacks, cost: sacrifice(2, island) },
    ],
    ..oracle::LEVIATHAN
};
