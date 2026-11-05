use cantrip_api::*;
use cantrip_mechanics::lands::sub;
use cantrip_mechanics::mobilize;
use cantrip_mechanics::statics::your_of_subtype;
use cantrip_oracle as oracle;

/// "During your end step, Warrior tokens you control ..." (CR 604.1, 613.1f, 102.1): a token under the source's controller's
/// control that is a Warrior as the earlier layers made it, the type read through the words of Zurgo's text (CR 612.2), only
/// while the active player is that controller and the step is the end step. Asked again whenever characteristics are computed.
fn your_warrior_tokens_during_your_end_step(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let turn = q.state().turn;
    q.state().obj(affected).token
        && turn.step() == Step::End
        && turn.active == q.controller(source)
        && your_of_subtype::<{ sub(Subtype::Warrior) }>(q, source, affected, so_far)
}

/// "... have 'This token can't be sacrificed'" (CR 101.2, 701.21a, 604.1, layer 6): the core's `CantBeSacrificed` keyword, so
/// mobilize's own sacrifice at the beginning of the end step does nothing to them.
fn have_cant_be_sacrificed(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::CantBeSacrificed);
}

pub static CARD: CardDef = CardDef {
    abilities: &[mobilize::trigger::<2>("Mobilize 2 (Whenever this creature attacks, create two tapped and attacking 1/1 red Warrior creature tokens. Sacrifice them at the beginning of the next end step.)")],
    delayed: &[mobilize::SACRIFICE],
    statics: &[static_ability(
        Layer::Ability,
        your_warrior_tokens_during_your_end_step,
        have_cant_be_sacrificed,
        "During your end step, Warrior tokens you control have \"This token can't be sacrificed.\"",
    )],
    ..oracle::ZURGO_THUNDERS_DECREE
};

pub static EXTRA: &[&CardDef] = &[&mobilize::WARRIOR];
