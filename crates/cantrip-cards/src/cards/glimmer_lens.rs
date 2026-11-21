use cantrip_api::*;
use cantrip_mechanics::attach::equip;
use cantrip_mechanics::for_mirrodin;
use cantrip_mechanics::triggers::creature_attacks_with_another;
use cantrip_oracle as oracle;

/// "Whenever equipped creature and at least one other creature attack".
fn equipped_and_another_attack(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    creature_attacks_with_another(q, q.attached_to(source)?, event)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        for_mirrodin::TRIGGER,
        triggered(when(EventMask::ATTACKERS_DECLARED, equipped_and_another_attack), |g, item| cantrip_mechanics::effects::draw(g, item, 1), "Whenever equipped creature and at least one other creature attack, draw a card."),
        equip(mana("{1}{W}"), "Equip {1}{W}"),
    ],
    ..oracle::GLIMMER_LENS
};

pub static EXTRA: &[&CardDef] = &[&for_mirrodin::REBEL];
