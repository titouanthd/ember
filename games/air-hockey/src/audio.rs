//! Sound bank. Wraps `ember_stdlib::audio::AudioClip`.
//!
//! Every clip is optional: if a `.wav` file is missing, `load` returns
//! `None` for that slot and the game runs silently. This is intentional —
//! the repo does not ship binary assets in git, and the game must be
//! playable from a fresh checkout.

use std::path::{Path, PathBuf};

use ember_stdlib::audio::AudioClip;

use crate::systems::GameEvent;

/// All clips the game can play.
pub struct SoundBank {
    wall: Option<AudioClip>,
    paddle_soft: Option<AudioClip>,
    paddle_hard: Option<AudioClip>,
    goal: Option<AudioClip>,
    beep: Option<AudioClip>,
    beep_go: Option<AudioClip>,
    round_win: Option<AudioClip>,
    match_win: Option<AudioClip>,
}

impl SoundBank {
    /// Load every clip from `<manifest_dir>/assets/`. Missing files are
    /// recorded as `None` and the corresponding event is a no-op.
    pub async fn load(manifest_dir: &Path) -> Self {
        let dir = manifest_dir.join("assets");
        async fn try_load(dir: &Path, name: &str) -> Option<AudioClip> {
            let p: PathBuf = dir.join(name);
            AudioClip::load(&p).await.ok()
        }
        Self {
            wall: try_load(&dir, "wall.wav").await,
            paddle_soft: try_load(&dir, "paddle_soft.wav").await,
            paddle_hard: try_load(&dir, "paddle_hard.wav").await,
            goal: try_load(&dir, "goal.wav").await,
            beep: try_load(&dir, "beep.wav").await,
            beep_go: try_load(&dir, "beep_go.wav").await,
            round_win: try_load(&dir, "round.wav").await,
            match_win: try_load(&dir, "match.wav").await,
        }
    }

    /// Play the sound associated with a game event, if any.
    pub fn on_event(&self, ev: GameEvent) {
        match ev {
            GameEvent::Wall => self.play(&self.wall, 0.5),
            GameEvent::Paddle { impact_speed } => {
                // 900 px/s is the same boundary used by `systems::HARD_HIT_SPEED`.
                if impact_speed > 900.0 {
                    self.play(&self.paddle_hard, 0.9);
                } else {
                    self.play(&self.paddle_soft, 0.6);
                }
            }
            GameEvent::Goal(_) => self.play(&self.goal, 1.0),
            GameEvent::CountdownBeep => self.play(&self.beep, 0.7),
            GameEvent::CountdownGo => self.play(&self.beep_go, 1.0),
            GameEvent::RoundWon(_) => self.play(&self.round_win, 0.9),
            GameEvent::MatchWon(_) => self.play(&self.match_win, 1.0),
        }
    }

    fn play(&self, clip: &Option<AudioClip>, volume: f32) {
        if let Some(c) = clip {
            c.play_at(volume);
        }
    }
}

#[cfg(test)]
mod tests {
    // `AudioClip::load` requires an active macroquad context, so we cannot
    // construct a `SoundBank` in unit tests. What we *can* test is that the
    // event → slot mapping is total (no event is silently dropped) by
    // exhaustively matching on `GameEvent`. If a new variant is added and
    // not handled, this test stops compiling.
    use super::*;
    use crate::components::Side;

    #[test]
    fn test_all_events_are_handled() {
        for ev in [
            GameEvent::Wall,
            GameEvent::Paddle { impact_speed: 100.0 },
            GameEvent::Paddle { impact_speed: 2000.0 },
            GameEvent::Goal(Side::Left),
            GameEvent::Goal(Side::Right),
            GameEvent::CountdownBeep,
            GameEvent::CountdownGo,
            GameEvent::RoundWon(Side::Left),
            GameEvent::MatchWon(Side::Right),
        ] {
            let bank = SoundBank {
                wall: None,
                paddle_soft: None,
                paddle_hard: None,
                goal: None,
                beep: None,
                beep_go: None,
                round_win: None,
                match_win: None,
            };
            bank.on_event(ev);
        }
    }
}