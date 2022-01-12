use crate::matter::{
    Direction, MatterCharacteristic, MatterDefinition, MatterDefinitions, MatterReaction,
    MatterState,
};

pub const MATTER_EMPTY: u32 = 0;
pub const MATTER_SAND: u32 = 1;
pub const MATTER_WATER: u32 = 2;
pub const MATTER_LAVA: u32 = 3;
pub const MATTER_ROCK: u32 = 4;
pub const MATTER_ICE: u32 = 5;
pub const MATTER_GLASS: u32 = 6;
pub const MATTER_WOOD: u32 = 7;
pub const MATTER_STEAM: u32 = 8;
pub const MATTER_SMOKE: u32 = 9;
pub const MATTER_GAS: u32 = 10;
pub const MATTER_FIRE: u32 = 11;
pub const MATTER_ACID: u32 = 12;
pub const MATTER_ERASE: u32 = 13;

pub fn default_matter_definitions() -> MatterDefinitions {
    MatterDefinitions {
        empty: MATTER_EMPTY,
        definitions: vec![
            MatterDefinition {
                id: MATTER_EMPTY,
                name: "Empty".to_string(),
                color: 0x0,
                weight: 0.0,
                state: MatterState::Empty,
                dispersion: 0,
                characteristics: MatterCharacteristic::empty(),
                reactions: [
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_SAND,
                name: "Sand".to_string(),
                color: 0xc2b280ff,
                weight: 1.5,
                state: MatterState::Powder,
                dispersion: 0,
                characteristics: (MatterCharacteristic::MELTS | MatterCharacteristic::CORRODES),
                reactions: [
                    MatterReaction {
                        reacts: MatterCharacteristic::MELTING,
                        direction: Direction::ALL,
                        probability: 0.6,
                        becomes: MATTER_GLASS,
                    },
                    MatterReaction {
                        reacts: MatterCharacteristic::CORROSIVE,
                        direction: Direction::ALL,
                        probability: 0.05,
                        becomes: MATTER_EMPTY,
                    },
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_WATER,
                name: "Water".to_string(),
                color: 0x1ca3ecff,
                weight: 1.0,
                state: MatterState::Liquid,
                dispersion: 10,
                characteristics: (MatterCharacteristic::RUSTING
                    | MatterCharacteristic::COOLING
                    | MatterCharacteristic::FREEZES
                    | MatterCharacteristic::VAPORIZES),
                reactions: [
                    MatterReaction {
                        reacts: (MatterCharacteristic::MELTING
                            | MatterCharacteristic::BURNING
                            | MatterCharacteristic::CORROSIVE),
                        direction: Direction::ALL,
                        probability: 0.6,
                        becomes: MATTER_STEAM,
                    },
                    MatterReaction {
                        reacts: (MatterCharacteristic::FREEZING),
                        direction: Direction::ALL,
                        probability: 0.005,
                        becomes: MATTER_ICE,
                    },
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_LAVA,
                name: "Lava".to_string(),
                color: 0xf7342bff,
                weight: 2.5,
                state: MatterState::Liquid,
                dispersion: 2,
                characteristics: (MatterCharacteristic::MELTING
                    | MatterCharacteristic::BURNING
                    | MatterCharacteristic::FREEZES
                    | MatterCharacteristic::COOLS),
                reactions: [
                    MatterReaction {
                        reacts: (MatterCharacteristic::FREEZING | MatterCharacteristic::COOLING),
                        direction: Direction::ALL,
                        probability: 0.5,
                        becomes: MATTER_ROCK,
                    },
                    // After melting or burning, some lava disappears.
                    MatterReaction {
                        reacts: (MatterCharacteristic::MELTS | MatterCharacteristic::BURNS),
                        direction: Direction::ALL,
                        probability: 0.6,
                        becomes: MATTER_EMPTY,
                    },
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_ROCK,
                name: "Rock".to_string(),
                color: 0x87898eff,
                weight: 2.5,
                state: MatterState::SolidGravity,
                dispersion: 0,
                characteristics: (MatterCharacteristic::CORRODES),
                reactions: [
                    MatterReaction {
                        reacts: (MatterCharacteristic::CORROSIVE),
                        direction: Direction::ALL,
                        probability: 0.05,
                        becomes: MATTER_EMPTY,
                    },
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_ICE,
                name: "Ice".to_string(),
                color: 0xb9e8eaff,
                weight: 1.0,
                state: MatterState::Solid,
                dispersion: 0,
                // Ice freezes others. Ice melts
                characteristics: (MatterCharacteristic::FREEZING | MatterCharacteristic::MELTS),
                reactions: [
                    MatterReaction {
                        reacts: (MatterCharacteristic::MELTING
                            | MatterCharacteristic::BURNING
                            | MatterCharacteristic::CORROSIVE),
                        direction: Direction::ALL,
                        probability: 0.4,
                        becomes: MATTER_WATER,
                    },
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_GLASS,
                name: "Glass".to_string(),
                color: 0xf6feffff,
                weight: 1.5,
                state: MatterState::SolidGravity,
                dispersion: 0,
                characteristics: (MatterCharacteristic::CORRODES),
                reactions: [
                    MatterReaction {
                        reacts: (MatterCharacteristic::CORROSIVE),
                        direction: Direction::ALL,
                        probability: 0.05,
                        becomes: MATTER_EMPTY,
                    },
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_WOOD,
                name: "Wood".to_string(),
                color: 0xba8c63ff,
                weight: 0.4,
                state: MatterState::Solid,
                dispersion: 0,
                characteristics: (MatterCharacteristic::BURNS | MatterCharacteristic::CORRODES),
                reactions: [
                    MatterReaction::becomes_on_touch_below(
                        0.4,
                        MatterCharacteristic::MELTING | MatterCharacteristic::BURNING,
                        MATTER_FIRE,
                    ),
                    MatterReaction::becomes_on_touch_below(
                        0.2,
                        MatterCharacteristic::MELTING | MatterCharacteristic::BURNING,
                        MATTER_SMOKE,
                    ),
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::CORROSIVE,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::becomes_on_touch(
                        0.05,
                        MatterCharacteristic::MELTING | MatterCharacteristic::BURNING,
                        MATTER_FIRE,
                    ),
                ],
            },
            MatterDefinition {
                id: MATTER_STEAM,
                name: "Steam".to_string(),
                color: 0x889a9eff,
                weight: 0.1,
                state: MatterState::Gas,
                dispersion: 5,
                reactions: [
                    MatterReaction::dies(0.005, MATTER_EMPTY),
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
                ..MatterDefinition::zero()
            },
            MatterDefinition {
                id: MATTER_SMOKE,
                name: "Smoke".to_string(),
                color: 0x7a7a7aff,
                weight: 0.1,
                state: MatterState::Gas,
                dispersion: 5,
                reactions: [
                    MatterReaction::dies(0.005, MATTER_EMPTY),
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
                ..MatterDefinition::zero()
            },
            MatterDefinition {
                id: MATTER_GAS,
                name: "Gas".to_string(),
                color: 0x92cd00ff,
                weight: 0.1,
                state: MatterState::Gas,
                dispersion: 5,
                reactions: [
                    MatterReaction::dies(0.005, MATTER_EMPTY),
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
                ..MatterDefinition::zero()
            },
            MatterDefinition {
                id: MATTER_FIRE,
                name: "Fire".to_string(),
                color: 0xe25822ff,
                weight: 0.0,
                state: MatterState::Energy,
                dispersion: 0,
                characteristics: (MatterCharacteristic::BURNING),
                reactions: [
                    // Better looking fire with a chance to disappear
                    MatterReaction::dies(0.2, MATTER_EMPTY),
                    MatterReaction::becomes_on_touch_below(
                        0.2,
                        MatterCharacteristic::BURNS,
                        MATTER_FIRE,
                    ),
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_ACID,
                name: "Acid".to_string(),
                color: 0xb0bf1aff,
                weight: 1.0,
                state: MatterState::Liquid,
                dispersion: 5,
                characteristics: (MatterCharacteristic::CORROSIVE | MatterCharacteristic::BURNS),
                reactions: [
                    // After corroding, acid can disappear. So when acid touches something that corrodes
                    MatterReaction {
                        reacts: (MatterCharacteristic::CORRODES),
                        direction: Direction::ALL,
                        probability: 0.2,
                        becomes: MATTER_EMPTY,
                    },
                    MatterReaction {
                        reacts: (MatterCharacteristic::BURNING),
                        direction: Direction::ALL,
                        probability: 0.4,
                        becomes: MATTER_FIRE,
                    }, // Acid also disappears over time... like gases
                    MatterReaction::dies(0.005, MATTER_EMPTY),
                    MatterReaction::becomes_on_touch(
                        1.0,
                        MatterCharacteristic::ERASER,
                        MATTER_EMPTY,
                    ),
                    MatterReaction::zero(),
                ],
            },
            MatterDefinition {
                id: MATTER_ERASE,
                name: "Erase".to_string(),
                color: 0x0,
                weight: 0.0,
                state: MatterState::Energy,
                dispersion: 0,
                characteristics: (MatterCharacteristic::ERASER),
                reactions: [
                    // Dies instantly
                    MatterReaction::dies(1.0, MATTER_EMPTY),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                    MatterReaction::zero(),
                ],
            },
        ],
    }
}
