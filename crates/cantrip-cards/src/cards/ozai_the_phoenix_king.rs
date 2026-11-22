use cantrip_api::*;
use cantrip_mechanics::firebending;
use cantrip_oracle as oracle;

/// "If you would lose unspent mana, that mana becomes red instead" (CR 106.4, 614.1a): the controller's mana only.
fn your_unspent_mana_becomes_red(_: &dyn Query, asker: &Asker, player: PlayerId) -> Option<Mana> {
    (player == asker.controller).then_some(Mana::R)
}

/// "... as long as you have six or more unspent mana" (CR 106.4, 604.2): the source itself, counting everything in its
/// controller's pool, the restricted mana kept apart from it included. The mana that lasts past its step is part of the pool.
fn itself_with_six_unspent_mana(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let state = q.state();
    let you = state.obj(source).controller;
    let unspent = state.player(you).pool.total() + state.restricted_of(you).map(|(_, pool)| pool.total()).sum::<u32>();
    source == affected && unspent >= 6
}

fn have_flying_and_indestructible(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Flying).with(Keyword::Indestructible);
}

pub static CARD: CardDef = CardDef {
    abilities: &[firebending::trigger::<4>("Firebending 4 (Whenever this creature attacks, add {R}{R}{R}{R}. This mana lasts until end of combat.)")],
    statics: &[static_ability(Layer::Ability, itself_with_six_unspent_mana, have_flying_and_indestructible, "Ozai has flying and indestructible as long as you have six or more unspent mana.")],
    mana_lost_becomes: &[your_unspent_mana_becomes_red],
    ..oracle::OZAI_THE_PHOENIX_KING
};
