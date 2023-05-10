use serde::{Deserialize, Serialize};

use crate::matter::{Direction, MatterCharacteristic, MatterState};

/// If you touch this, also change shaders...
pub const MAX_TRANSITIONS: u32 = 5;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct MatterReaction {
    pub reacts: MatterCharacteristic,
    pub direction: Direction,
    pub probability: f32,
    pub becomes: u32,
}

impl MatterReaction {
    pub fn zero() -> Self {
        MatterReaction {
            reacts: MatterCharacteristic::empty(),
            direction: Direction::NONE,
            probability: 0.0,
            becomes: 0,
        }
    }

    pub fn dies(p: f32, empty_matter: u32) -> Self {
        MatterReaction {
            reacts: MatterCharacteristic::empty(),
            direction: Direction::ALL,
            probability: p,
            becomes: empty_matter,
        }
    }

    pub fn becomes_on_touch(
        p: f32,
        touch_characteristic: MatterCharacteristic,
        becomes_matter: u32,
    ) -> Self {
        MatterReaction {
            reacts: touch_characteristic,
            direction: Direction::ALL,
            probability: p,
            becomes: becomes_matter,
        }
    }

    // Good for e.g. fire
    pub fn becomes_on_touch_below(
        p: f32,
        touch_characteristic: MatterCharacteristic,
        becomes_matter: u32,
    ) -> Self {
        MatterReaction {
            reacts: touch_characteristic,
            direction: (Direction::DOWN
                | Direction::DOWN_LEFT
                | Direction::DOWN_RIGHT
                | Direction::RIGHT
                | Direction::LEFT),
            probability: p,
            becomes: becomes_matter,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatterDefinition {
    pub id: u32,
    pub name: String,
    pub color: u32,
    pub weight: f32,
    pub state: MatterState,
    pub dispersion: u32,
    /// What are the characteristics of matter?
    /// - Water: "Cools", "Rusts"
    /// - Acid: "Corrodes".
    /// Think of it like: "What does this do to others?"
    pub characteristics: MatterCharacteristic,
    /// How does matter react to neighbor characteristics?
    /// - Example: "Water becomes ice on probability x if touches one that freezes".
    /// - Example: "Acid might become empty on probability x if touches a material it corroded (corroding)".
    /// Probability will affect the speed at which matter changes
    pub reactions: [MatterReaction; MAX_TRANSITIONS as usize],
}

impl MatterDefinition {
    pub fn zero() -> Self {
        MatterDefinition {
            id: 0,
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
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatterDefinitions {
    pub definitions: Vec<MatterDefinition>,
    pub empty: u32,
}

impl MatterDefinitions {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn deserialize(data: &str) -> MatterDefinitions {
        let deserialized: MatterDefinitions = serde_json::from_str(data).unwrap();
        deserialized
    }
}

pub fn validate_matter_definitions(matter_definitions: &MatterDefinitions) {
    for (i, m) in matter_definitions.definitions.iter().enumerate() {
        if m.id != i as u32 {
            panic!(
                "Invalid matter definition, definition {}: id {} does not equal matter id index {}",
                m.name,
                { m.id },
                i as u32
            );
        }
        if m.reactions
            .iter()
            .any(|r| r.becomes >= matter_definitions.definitions.len() as u32)
        {
            panic!(
                "Matter reaction invalid for id: {}, name: {}. 'becomes' must not be larger than \
                 any id",
                m.id, m.name
            )
        }
    }
}
