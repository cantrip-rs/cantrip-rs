use cantrip_api::*;
use cantrip_mechanics::statics::itself_with_delirium as affects;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(NONCREATURE_SPELL_YOU_CAST, |g, item| cantrip_mechanics::effects::surveil(g, item, 1), "Whenever you cast a noncreature spell, surveil 1.")],
    statics: &[
        static_ability(Layer::PtModify, affects, cantrip_mechanics::statics::plus_pt::<2, 2>, "Delirium \u{2014} As long as there are four or more card types among cards in your graveyard, this creature gets +2/+2."),
        static_ability(Layer::Ability, affects, cantrip_mechanics::statics::have_flying, "Delirium \u{2014} As long as there are four or more card types among cards in your graveyard, this creature has flying."),
        static_ability(Layer::Ability, affects, cantrip_mechanics::statics::attacks_each_combat_if_able, "Delirium \u{2014} As long as there are four or more card types among cards in your graveyard, this creature attacks each combat if able."),
    ],
    ..oracle::DRAGONS_RAGE_CHANNELER
};
