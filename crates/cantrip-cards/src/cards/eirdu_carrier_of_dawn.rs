use cantrip_api::*;
use cantrip_mechanics::statics::{self, grant_delayed_ability, your_creatures};
use cantrip_mechanics::{effects, persist, triggers};
use cantrip_oracle as oracle;

/// "Each other nontoken creature you control" (CR 613.1f, layer 6): the creatures Isilu's controller controls, judged on what the
/// earlier layers made them, except Isilu itself and tokens.
fn other_nontoken_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    affected != source && !q.state().obj(affected).token && your_creatures(q, source, affected, so_far)
}

/// "At the beginning of your first main phase, you may pay {B}. If you do, transform Eirdu."
fn pay_black(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{B}"))
}

/// "At the beginning of your first main phase, you may pay {W}. If you do, transform Isilu."
fn pay_white(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{W}"))
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_white, "At the beginning of your first main phase, you may pay {W}. If you do, transform Isilu.")],
    delayed: &[persist::trigger("Persist (When this creature dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.)")],
    statics: &[static_ability(
        Layer::Ability,
        other_nontoken_creatures_you_control,
        grant_delayed_ability,
        "Each other nontoken creature you control has persist. (When it dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.)",
    )],
    ..oracle::ISILU_CARRIER_OF_TWILIGHT
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_black, "At the beginning of your first main phase, you may pay {B}. If you do, transform Eirdu.")],
    grants_pay_with: &[statics::creature_spells_you_cast_have_convoke],
    back: Some(&BACK),
    ..oracle::EIRDU_CARRIER_OF_DAWN
};
