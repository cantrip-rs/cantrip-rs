use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Cosmic Awareness — As long as an opponent has cast a spell this turn, you may cast spells as though they had flash" (CR 601.3,
/// 702.8a): its controller's spells, from whatever zone they cast them from and whatever the spell is, while some opponent of its
/// controller has cast a spell this turn - asked of the history each time (CR 611.3a), so it starts the moment that opponent's
/// spell is cast, stays when it has resolved or been countered, and is over when the turn ends.
fn spells_you_cast_after_an_opponent_cast_one(q: &dyn Query, asker: &Asker, _: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller && q.state().opponents(asker.controller).any(|opponent| q.history().casts_by(opponent).next().is_some())
}

/// Flying and vigilance are core keywords.
pub static CARD: CardDef = CardDef { flash_rules: &[spells_you_cast_after_an_opponent_cast_one], ..oracle::CAPTAIN_MAR_VELL_SPACE_BORN };
