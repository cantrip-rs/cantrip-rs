use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "You can't get poison counters" (CR 122.1, 101.2): none are given, by infect damage or by an effect, and the ones its
/// controller already has stay (the 2011-06-01 ruling).
fn you_cant_get_poison(_: &dyn Query, asker: &Asker, player: PlayerId, kind: PlayerCounter) -> bool {
    player == asker.controller && kind == PlayerCounter::Poison
}

/// "... lose infect" (CR 613.1f): layer 6, so infect given in a later timestamp applies over it (the Glistening Oil ruling).
fn lose_infect(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = Keywords(chars.keywords.0 & !(1 << Keyword::Infect as u8));
}

pub static CARD: CardDef = CardDef {
    player_counter_bans: &[you_cant_get_poison],
    counter_bans: &[statics::no_minus_counters_on_your_creatures],
    statics: &[static_ability(Layer::Ability, statics::your_opponents_creatures, lose_infect, "Creatures your opponents control lose infect.")],
    ..oracle::MELIRA_SYLVOK_OUTCAST
};
