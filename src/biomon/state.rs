use super::theme::{AccessibilityConfig, PanelId};

// ───────────────────────── Suit Condition & Repair Domain ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SuitFault {
    Nominal,
    GloveTear,
    ScrubberDrift,
    CoreBreach,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Repairability {
    Intact,
    FieldRepairable,
    AirlockRequired,
}

pub(crate) struct SuitZoneStatus {
    pub zone: &'static str,
    pub integrity_pct: u8,
    pub repairability: Repairability,
    pub action: &'static str,
}

impl SuitFault {
    pub(crate) const ALL: [SuitFault; 4] = [
        Self::Nominal,
        Self::GloveTear,
        Self::ScrubberDrift,
        Self::CoreBreach,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Nominal => "Suit 100%",
            Self::GloveTear => "Glove seam tear",
            Self::ScrubberDrift => "Scrubber clog",
            Self::CoreBreach => "Visor / hull crack",
        }
    }

    pub(crate) fn sub(self) -> &'static str {
        match self {
            Self::Nominal => "Nominal",
            Self::GloveTear => "Repairable",
            Self::ScrubberDrift => "Repairable",
            Self::CoreBreach => "Unrepairable",
        }
    }
}

// ───────────────────────── Mission Telemetry State ─────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Main,
    Input,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Launch,
    Orbit,
    Eva,
    ReEntry,
}

impl Phase {
    pub(crate) const ALL: [Phase; 4] = [Phase::Launch, Phase::Orbit, Phase::Eva, Phase::ReEntry];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Launch => "Launch",
            Self::Orbit => "Orbit",
            Self::Eva => "EVA",
            Self::ReEntry => "Re-entry",
        }
    }

    pub(crate) fn base_hr(self) -> i32 {
        match self {
            Self::Launch => 124,
            Self::Orbit => 88,
            Self::Eva => 138,
            Self::ReEntry => 132,
        }
    }

    pub(crate) fn hr_bounds(self) -> (i32, i32) {
        match self {
            Self::Launch => (100, 140),
            Self::Orbit => (55, 95),
            Self::Eva => (110, 155),
            Self::ReEntry => (90, 135),
        }
    }

    pub(crate) fn hr_range(self) -> &'static str {
        match self {
            Self::Launch => "100–140 bpm",
            Self::Orbit => "55–95 bpm",
            Self::Eva => "110–155 bpm",
            Self::ReEntry => "90–135 bpm",
        }
    }

    pub(crate) fn bp(self) -> &'static str {
        match self {
            Self::Launch => "132/84",
            Self::Orbit => "118/76",
            Self::Eva => "138/86",
            Self::ReEntry => "102/64",
        }
    }

    pub(crate) fn kcal_rate(self) -> f32 {
        match self {
            Self::Launch => 3.4,
            Self::Orbit => 1.8,
            Self::Eva => 6.8,
            Self::ReEntry => 3.1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hazard {
    Nominal,
    Radiation,
    HighCo2,
    Hypoxia,
}

impl Hazard {
    pub(crate) const ALL: [Hazard; 4] = [
        Hazard::Nominal,
        Hazard::Radiation,
        Hazard::HighCo2,
        Hazard::Hypoxia,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Nominal => "Nominal",
            Self::Radiation => "Solar radiation spike",
            Self::HighCo2 => "High suit CO2",
            Self::Hypoxia => "Low tissue oxygen",
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Nominal => "All levels safe",
            Self::Radiation => "Radiation alert",
            Self::HighCo2 => "High CO2",
            Self::Hypoxia => "Low O2",
        }
    }

    pub(crate) fn rad(self) -> &'static str {
        match self {
            Self::Radiation => "4.85 mSv/d",
            _ => "0.42 mSv/d",
        }
    }

    pub(crate) fn rad_val(self) -> &'static str {
        match self {
            Self::Radiation => "4.85",
            _ => "0.42",
        }
    }

    pub(crate) fn rad_ratio(self) -> f32 {
        match self {
            Self::Radiation => 0.96,
            _ => 0.21,
        }
    }

    pub(crate) fn co2(self) -> &'static str {
        match self {
            Self::HighCo2 => "5.4 mmHg",
            Self::Hypoxia => "3.4 mmHg",
            _ => "2.8 mmHg",
        }
    }

    pub(crate) fn co2_val(self) -> &'static str {
        match self {
            Self::HighCo2 => "5.4",
            Self::Hypoxia => "3.4",
            _ => "2.8",
        }
    }

    pub(crate) fn co2_ratio(self) -> f32 {
        match self {
            Self::HighCo2 => 0.90,
            Self::Hypoxia => 0.56,
            _ => 0.46,
        }
    }

    pub(crate) fn o2(self) -> &'static str {
        match self {
            Self::HighCo2 => "96%",
            Self::Hypoxia => "89%",
            _ => "98%",
        }
    }

    pub(crate) fn o2_ratio(self) -> f32 {
        match self {
            Self::HighCo2 => 0.90,
            Self::Hypoxia => 0.40,
            _ => 0.96,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavWaypoint {
    Habitat,
    Airlock,
}

impl NavWaypoint {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Habitat => "Habitat",
            Self::Airlock => "Airlock",
        }
    }

    pub(crate) fn bearing_deg(self) -> f32 {
        match self {
            Self::Habitat => 42.0,
            Self::Airlock => 312.0,
        }
    }

    pub(crate) fn cardinal(self) -> &'static str {
        match self {
            Self::Habitat => "NE",
            Self::Airlock => "NW",
        }
    }
}

pub(crate) struct BiomonState {
    pub mode: Mode,
    pub phase: Phase,
    pub online: bool,
    pub buffered_packets: u32,
    pub meal_eaten_pct: u32,
    pub water_ml: u32,
    pub nacl_count: u32,
    pub hazard: Hazard,
    pub suit_fault: SuitFault,
    pub waypoint: NavWaypoint,
    pub distance_m: u32,
    pub compass_heading_deg: f32,
    pub calorie_deficit: u32,
    pub sweat_deficit_ml: u32,
    pub live_hr: i32,
    pub active_panel: Option<PanelId>,
    pub a11y: AccessibilityConfig,
}

impl Default for BiomonState {
    fn default() -> Self {
        Self {
            mode: Mode::Main,
            phase: Phase::Orbit,
            online: true,
            buffered_packets: 0,
            meal_eaten_pct: 100,
            water_ml: 0,
            nacl_count: 0,
            hazard: Hazard::Nominal,
            suit_fault: SuitFault::Nominal,
            waypoint: NavWaypoint::Habitat,
            distance_m: 222,
            compass_heading_deg: 42.0,
            calorie_deficit: 120,
            sweat_deficit_ml: 85,
            live_hr: 88,
            active_panel: None,
            a11y: AccessibilityConfig::default(),
        }
    }
}

impl BiomonState {
    pub(crate) fn relative_bearing_deg(&self) -> f32 {
        (self.waypoint.bearing_deg() - self.compass_heading_deg + 360.0) % 360.0
    }

    pub(crate) fn set_waypoint(&mut self, wp: NavWaypoint) {
        self.waypoint = wp;
        self.distance_m = match wp {
            NavWaypoint::Habitat => 222,
            NavWaypoint::Airlock => 145,
        };
    }

    pub(crate) fn suit_zones(&self) -> [SuitZoneStatus; 5] {
        let helmet = if self.hazard == Hazard::Hypoxia {
            SuitZoneStatus {
                zone: "Helmet & O2",
                integrity_pct: 58,
                repairability: Repairability::FieldRepairable,
                action: "Field repair: switch to Backup O2 Pack",
            }
        } else if self.suit_fault == SuitFault::CoreBreach {
            SuitZoneStatus {
                zone: "Helmet & O2",
                integrity_pct: 32,
                repairability: Repairability::AirlockRequired,
                action: "NOT repairable in EVA — return to airlock",
            }
        } else {
            SuitZoneStatus {
                zone: "Helmet & O2",
                integrity_pct: 100,
                repairability: Repairability::Intact,
                action: "4.3 psi seal intact",
            }
        };

        let torso = if self.suit_fault == SuitFault::CoreBreach {
            SuitZoneStatus {
                zone: "Torso shell",
                integrity_pct: 28,
                repairability: Repairability::AirlockRequired,
                action: "Pressure bladder breach — abort EVA",
            }
        } else {
            SuitZoneStatus {
                zone: "Torso shell",
                integrity_pct: 100,
                repairability: Repairability::Intact,
                action: "Cooling loop & hull OK",
            }
        };

        let scrubber =
            if self.hazard == Hazard::HighCo2 || self.suit_fault == SuitFault::ScrubberDrift {
                SuitZoneStatus {
                    zone: "PLSS scrubber",
                    integrity_pct: 52,
                    repairability: Repairability::FieldRepairable,
                    action: "Field repair: swap spare LiOH cartridge",
                }
            } else {
                SuitZoneStatus {
                    zone: "PLSS scrubber",
                    integrity_pct: 98,
                    repairability: Repairability::Intact,
                    action: "CO2 scrub flow nominal",
                }
            };

        let gloves = if self.suit_fault == SuitFault::GloveTear {
            SuitZoneStatus {
                zone: "Gloves & seams",
                integrity_pct: 64,
                repairability: Repairability::FieldRepairable,
                action: "Field repair: Kapton patch from thigh kit",
            }
        } else {
            SuitZoneStatus {
                zone: "Gloves & seams",
                integrity_pct: 100,
                repairability: Repairability::Intact,
                action: "Wrist bearing seals tight",
            }
        };

        let shield = if self.hazard == Hazard::Radiation {
            SuitZoneStatus {
                zone: "Rad shielding",
                integrity_pct: 45,
                repairability: Repairability::AirlockRequired,
                action: "Shelter needed — unrepairable outside",
            }
        } else {
            SuitZoneStatus {
                zone: "Rad shielding",
                integrity_pct: 100,
                repairability: Repairability::Intact,
                action: "Thermal & dust shield intact",
            }
        };

        [helmet, torso, scrubber, gloves, shield]
    }

    pub(crate) fn overall_suit_repairability(&self) -> Repairability {
        let zones = self.suit_zones();
        if zones
            .iter()
            .any(|z| z.repairability == Repairability::AirlockRequired)
        {
            Repairability::AirlockRequired
        } else if zones
            .iter()
            .any(|z| z.repairability == Repairability::FieldRepairable)
        {
            Repairability::FieldRepairable
        } else {
            Repairability::Intact
        }
    }

    pub(crate) fn apply_field_repair(&mut self) {
        if self.suit_fault == SuitFault::GloveTear || self.suit_fault == SuitFault::ScrubberDrift {
            self.suit_fault = SuitFault::Nominal;
        }
        if self.hazard == Hazard::HighCo2 || self.hazard == Hazard::Hypoxia {
            self.hazard = Hazard::Nominal;
        }
    }

    pub(crate) fn ration_short(&self) -> &'static str {
        if self.phase == Phase::ReEntry {
            "1.15 L + 9x NaCl"
        } else if self.hazard == Hazard::HighCo2 {
            "Low-Na+ salmon"
        } else if self.meal_eaten_pct < 100 || self.phase == Phase::Eva {
            "High-K+ fajita"
        } else {
            "Balanced #4"
        }
    }

    pub(crate) fn set_meal(&mut self, pct: u32) {
        self.meal_eaten_pct = pct;
        let eva_base = if self.phase == Phase::Eva { 410 } else { 120 };
        self.calorie_deficit = ((100 - pct) as f32 * 6.5).round() as u32 + eva_base;
    }

    pub(crate) fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.live_hr = phase.base_hr();
        if phase == Phase::Eva {
            self.calorie_deficit = 410;
            self.sweat_deficit_ml = 850;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suit_repairability_reflects_fault_and_clears_on_field_repair() {
        let mut s = BiomonState::default();
        assert_eq!(s.overall_suit_repairability(), Repairability::Intact);

        s.suit_fault = SuitFault::GloveTear;
        assert_eq!(s.overall_suit_repairability(), Repairability::FieldRepairable);
        s.apply_field_repair();
        assert_eq!(s.overall_suit_repairability(), Repairability::Intact);

        s.suit_fault = SuitFault::CoreBreach;
        assert_eq!(s.overall_suit_repairability(), Repairability::AirlockRequired);
    }

    #[test]
    fn waypoint_switch_updates_distance_and_bearing() {
        let mut s = BiomonState::default();
        assert_eq!(s.distance_m, 222);
        s.set_waypoint(NavWaypoint::Airlock);
        assert_eq!(s.distance_m, 145);
        assert_eq!(s.waypoint.bearing_deg(), 312.0);
    }
}
