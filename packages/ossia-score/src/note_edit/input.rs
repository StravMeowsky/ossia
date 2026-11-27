//! Note-input actions from the protocol.
use super::*;

impl NoteValue {
    fn from_input(value: ossia_types::NoteInputValue) -> Result<Self, AppError> {
        if value.log > 7 || value.dots > 3 {
            return Err(invalid("unsupported note value"));
        }
        Ok(Self {
            log: value.log,
            dots: value.dots,
        })
    }
}

impl TryFrom<&ossia_types::NoteInputAction> for NoteEditOp {
    type Error = AppError;

    /// Every action except `Restore`, which replaces the whole source instead.
    fn try_from(action: &ossia_types::NoteInputAction) -> Result<Self, AppError> {
        use ossia_types::NoteInputAction as A;
        Ok(match action {
            A::Enter { step, value, chord } => {
                if *step > 6 {
                    return Err(invalid("unknown step"));
                }
                Self::Enter {
                    step: *step,
                    value: NoteValue::from_input(*value)?,
                    chord: *chord,
                }
            }
            A::Rest { value } => Self::Rest {
                value: NoteValue::from_input(*value)?,
            },
            A::Step { steps } => Self::Step { steps: *steps },
            A::Transpose { semitones } => Self::Transpose {
                semitones: *semitones,
            },
            A::SetValue { value } => Self::SetValue {
                value: NoteValue::from_input(*value)?,
            },
            A::Delete => Self::Delete,
            A::ToggleTie => Self::ToggleTie,
            A::ToggleSlur => Self::ToggleSlur,
            A::Dynamic { mark } => Self::Dynamic { mark: mark.clone() },
            A::Restore { .. } => return Err(invalid("restore is not a note edit")),
        })
    }
}
