#![forbid(unsafe_code)]

//! Long-duration mission planning and execution with ternary progress tracking.

/// Ternary status: behind schedule, on track, ahead of schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ternary {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}

impl Ternary {
    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Ternary::Neg),
            0 => Some(Ternary::Zero),
            1 => Some(Ternary::Pos),
            _ => None,
        }
    }

    pub fn to_i8(self) -> i8 {
        self as i8
    }
}

/// Unique identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VoyageId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaypointId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoyageStatus {
    Planned,
    Active,
    Completed,
    Aborted,
}

/// A multi-room journey with waypoints.
#[derive(Clone, Debug)]
pub struct Voyage {
    pub id: VoyageId,
    pub label: String,
    pub waypoints: Vec<Waypoint>,
    pub status: VoyageStatus,
    pub current_waypoint: Option<WaypointId>,
    pub resources_total: u64,
    pub resources_remaining: u64,
}

impl Voyage {
    pub fn new(id: u64, label: &str, resources: u64) -> Self {
        Voyage {
            id: VoyageId(id),
            label: label.to_string(),
            waypoints: Vec::new(),
            status: VoyageStatus::Planned,
            current_waypoint: None,
            resources_total: resources,
            resources_remaining: resources,
        }
    }

    pub fn add_waypoint(&mut self, waypoint: Waypoint) {
        self.waypoints.push(waypoint);
    }

    /// Start the voyage (transitions from Planned to Active).
    pub fn start(&mut self) -> bool {
        if self.status == VoyageStatus::Planned && !self.waypoints.is_empty() {
            self.status = VoyageStatus::Active;
            self.current_waypoint = Some(self.waypoints[0].id);
            true
        } else {
            false
        }
    }

    /// Advance to the next waypoint.
    pub fn advance(&mut self) -> bool {
        if self.status != VoyageStatus::Active {
            return false;
        }
        if let Some(current) = self.current_waypoint {
            let idx = self.waypoints.iter().position(|w| w.id == current);
            if let Some(i) = idx {
                // Mark current as reached
                self.waypoints[i].reached = true;
                if i + 1 < self.waypoints.len() {
                    self.current_waypoint = Some(self.waypoints[i + 1].id);
                    true
                } else {
                    self.status = VoyageStatus::Completed;
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Abort the voyage.
    pub fn abort(&mut self) -> bool {
        if self.status == VoyageStatus::Active {
            self.status = VoyageStatus::Aborted;
            true
        } else {
            false
        }
    }

    /// Get progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.waypoints.is_empty() {
            return 0.0;
        }
        let reached = self.waypoints.iter().filter(|w| w.reached).count();
        reached as f64 / self.waypoints.len() as f64
    }

    /// Consume resources.
    pub fn consume_resources(&mut self, amount: u64) -> bool {
        if amount <= self.resources_remaining {
            self.resources_remaining -= amount;
            true
        } else {
            false
        }
    }
}

/// A stop along the voyage.
#[derive(Clone, Debug)]
pub struct Waypoint {
    pub id: WaypointId,
    pub label: String,
    pub expected_duration: u64, // time units
    pub reached: bool,
    pub resource_cost: u64,
}

impl Waypoint {
    pub fn new(id: u64, label: &str, expected_duration: u64, resource_cost: u64) -> Self {
        Waypoint {
            id: WaypointId(id),
            label: label.to_string(),
            expected_duration,
            reached: false,
            resource_cost,
        }
    }
}

/// Record of events during a voyage.
#[derive(Clone, Debug)]
pub struct VoyageLog {
    pub entries: Vec<LogEntry>,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: u64,
    pub event: String,
    pub details: String,
}

impl VoyageLog {
    pub fn new() -> Self {
        VoyageLog { entries: Vec::new() }
    }

    pub fn log(&mut self, timestamp: u64, event: &str, details: &str) {
        self.entries.push(LogEntry {
            timestamp,
            event: event.to_string(),
            details: details.to_string(),
        });
    }

    pub fn entries_since(&self, timestamp: u64) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.timestamp >= timestamp).collect()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Predicts arrival time and resource usage.
#[derive(Clone, Debug)]
pub struct VoyageEstimator;

impl VoyageEstimator {
    /// Estimate remaining time based on waypoints left.
    pub fn estimate_remaining_time(voyage: &Voyage) -> u64 {
        voyage.waypoints.iter()
            .filter(|w| !w.reached)
            .map(|w| w.expected_duration)
            .sum()
    }

    /// Estimate remaining resource cost.
    pub fn estimate_remaining_resources(voyage: &Voyage) -> u64 {
        voyage.waypoints.iter()
            .filter(|w| !w.reached)
            .map(|w| w.resource_cost)
            .sum()
    }

    /// Check if the voyage has enough resources to complete.
    pub fn can_complete(voyage: &Voyage) -> bool {
        Self::estimate_remaining_resources(voyage) <= voyage.resources_remaining
    }

    /// Get ternary status: behind/on track/ahead based on resource usage vs progress.
    pub fn status(voyage: &Voyage) -> Ternary {
        if voyage.waypoints.is_empty() {
            return Ternary::Zero;
        }
        let progress = voyage.progress();
        let resource_fraction = if voyage.resources_total == 0 {
            0.0
        } else {
            1.0 - (voyage.resources_remaining as f64 / voyage.resources_total as f64)
        };
        if progress > resource_fraction + 0.1 {
            Ternary::Pos // ahead of schedule
        } else if progress < resource_fraction - 0.1 {
            Ternary::Neg // behind schedule
        } else {
            Ternary::Zero // on track
        }
    }
}

/// Adjusts course during the voyage.
#[derive(Clone, Debug)]
pub struct VoyageNavigator {
    voyage: Voyage,
}

impl VoyageNavigator {
    pub fn new(voyage: Voyage) -> Self {
        VoyageNavigator { voyage }
    }

    /// Insert a waypoint after a given waypoint.
    pub fn insert_waypoint_after(&mut self, after_id: WaypointId, waypoint: Waypoint) -> bool {
        let idx = self.voyage.waypoints.iter().position(|w| w.id == after_id);
        if let Some(i) = idx {
            self.voyage.waypoints.insert(i + 1, waypoint);
            true
        } else {
            false
        }
    }

    /// Remove a waypoint that hasn't been reached yet.
    pub fn remove_waypoint(&mut self, id: WaypointId) -> bool {
        let idx = self.voyage.waypoints.iter().position(|w| w.id == id);
        if let Some(i) = idx {
            if !self.voyage.waypoints[i].reached {
                self.voyage.waypoints.remove(i);
                return true;
            }
        }
        false
    }

    /// Reroute: replace an unreached waypoint with a new one.
    pub fn reroute(&mut self, old_id: WaypointId, new_waypoint: Waypoint) -> bool {
        let idx = self.voyage.waypoints.iter().position(|w| w.id == old_id);
        if let Some(i) = idx {
            if !self.voyage.waypoints[i].reached {
                self.voyage.waypoints[i] = new_waypoint;
                return true;
            }
        }
        false
    }

    pub fn voyage(&self) -> &Voyage {
        &self.voyage
    }
}

/// Verifies mission success criteria.
#[derive(Clone, Debug)]
pub struct VoyageCompletion {
    pub required_waypoints: Vec<WaypointId>,
    pub minimum_resources_remaining: u64,
}

impl VoyageCompletion {
    pub fn new(required: Vec<WaypointId>, min_resources: u64) -> Self {
        VoyageCompletion {
            required_waypoints: required,
            minimum_resources_remaining: min_resources,
        }
    }

    /// Check if the voyage meets all completion criteria.
    pub fn verify(&self, voyage: &Voyage) -> CompletionResult {
        let mut missing: Vec<WaypointId> = Vec::new();
        for wp_id in &self.required_waypoints {
            let reached = voyage.waypoints.iter()
                .any(|w| &w.id == wp_id && w.reached);
            if !reached {
                missing.push(*wp_id);
            }
        }
        let resources_ok = voyage.resources_remaining >= self.minimum_resources_remaining;
        let success = missing.is_empty() && resources_ok && voyage.status == VoyageStatus::Completed;
        CompletionResult {
            success,
            missing_waypoints: missing,
            resources_ok,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompletionResult {
    pub success: bool,
    pub missing_waypoints: Vec<WaypointId>,
    pub resources_ok: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_values() {
        assert_eq!(Ternary::from_i8(-1), Some(Ternary::Neg));
        assert_eq!(Ternary::from_i8(0), Some(Ternary::Zero));
        assert_eq!(Ternary::from_i8(1), Some(Ternary::Pos));
    }

    #[test]
    fn test_voyage_creation() {
        let v = Voyage::new(1, "test voyage", 1000);
        assert_eq!(v.id, VoyageId(1));
        assert_eq!(v.status, VoyageStatus::Planned);
        assert_eq!(v.resources_total, 1000);
    }

    #[test]
    fn test_voyage_add_waypoint() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "stop-a", 10, 100));
        assert_eq!(v.waypoints.len(), 1);
    }

    #[test]
    fn test_voyage_start() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        assert!(v.start());
        assert_eq!(v.status, VoyageStatus::Active);
    }

    #[test]
    fn test_voyage_start_empty_fails() {
        let mut v = Voyage::new(1, "test", 1000);
        assert!(!v.start());
    }

    #[test]
    fn test_voyage_advance() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.add_waypoint(Waypoint::new(2, "b", 10, 100));
        v.start();
        assert!(v.advance());
        assert_eq!(v.current_waypoint, Some(WaypointId(2)));
    }

    #[test]
    fn test_voyage_advance_completes() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.start();
        assert!(!v.advance()); // no more waypoints
        assert_eq!(v.status, VoyageStatus::Completed);
    }

    #[test]
    fn test_voyage_abort() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.start();
        assert!(v.abort());
        assert_eq!(v.status, VoyageStatus::Aborted);
    }

    #[test]
    fn test_voyage_progress() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.add_waypoint(Waypoint::new(2, "b", 10, 100));
        v.add_waypoint(Waypoint::new(3, "c", 10, 100));
        v.start();
        v.advance(); // reaches a
        assert!((v.progress() - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_voyage_consume_resources() {
        let mut v = Voyage::new(1, "test", 1000);
        assert!(v.consume_resources(300));
        assert_eq!(v.resources_remaining, 700);
    }

    #[test]
    fn test_voyage_consume_resources_over() {
        let mut v = Voyage::new(1, "test", 100);
        assert!(!v.consume_resources(200));
        assert_eq!(v.resources_remaining, 100);
    }

    #[test]
    fn test_waypoint_creation() {
        let wp = Waypoint::new(1, "stop", 50, 200);
        assert_eq!(wp.id, WaypointId(1));
        assert_eq!(wp.expected_duration, 50);
        assert!(!wp.reached);
    }

    #[test]
    fn test_voyage_log() {
        let mut log = VoyageLog::new();
        log.log(100, "started", "voyage began");
        log.log(200, "arrived", "reached waypoint a");
        assert_eq!(log.entry_count(), 2);
    }

    #[test]
    fn test_voyage_log_entries_since() {
        let mut log = VoyageLog::new();
        log.log(100, "a", "first");
        log.log(200, "b", "second");
        log.log(300, "c", "third");
        assert_eq!(log.entries_since(200).len(), 2);
    }

    #[test]
    fn test_estimator_remaining_time() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.add_waypoint(Waypoint::new(2, "b", 20, 200));
        assert_eq!(VoyageEstimator::estimate_remaining_time(&v), 30);
    }

    #[test]
    fn test_estimator_remaining_resources() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.add_waypoint(Waypoint::new(2, "b", 20, 300));
        assert_eq!(VoyageEstimator::estimate_remaining_resources(&v), 400);
    }

    #[test]
    fn test_estimator_can_complete() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 300));
        assert!(VoyageEstimator::can_complete(&v));
    }

    #[test]
    fn test_estimator_cannot_complete() {
        let mut v = Voyage::new(1, "test", 100);
        v.add_waypoint(Waypoint::new(1, "a", 10, 300));
        assert!(!VoyageEstimator::can_complete(&v));
    }

    #[test]
    fn test_estimator_status_ahead() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.add_waypoint(Waypoint::new(2, "b", 10, 100));
        v.start();
        v.advance(); // reached a, 50% progress, used 0 resources from remaining
        // Status should be Pos since progress > resource usage fraction
        assert_eq!(VoyageEstimator::status(&v), Ternary::Pos);
    }

    #[test]
    fn test_navigator_insert_waypoint() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.add_waypoint(Waypoint::new(3, "c", 10, 100));
        let mut nav = VoyageNavigator::new(v);
        assert!(nav.insert_waypoint_after(WaypointId(1), Waypoint::new(2, "b", 10, 100)));
        assert_eq!(nav.voyage().waypoints.len(), 3);
    }

    #[test]
    fn test_navigator_remove_waypoint() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.add_waypoint(Waypoint::new(2, "b", 10, 100));
        let mut nav = VoyageNavigator::new(v);
        assert!(nav.remove_waypoint(WaypointId(2)));
        assert_eq!(nav.voyage().waypoints.len(), 1);
    }

    #[test]
    fn test_navigator_reroute() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        let mut nav = VoyageNavigator::new(v);
        assert!(nav.reroute(WaypointId(1), Waypoint::new(10, "a-new", 5, 50)));
        assert_eq!(nav.voyage().waypoints[0].label, "a-new");
    }

    #[test]
    fn test_completion_success() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.start();
        v.advance(); // completes
        let comp = VoyageCompletion::new(vec![WaypointId(1)], 0);
        let result = comp.verify(&v);
        assert!(result.success);
    }

    #[test]
    fn test_completion_missing_waypoint() {
        let mut v = Voyage::new(1, "test", 1000);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.add_waypoint(Waypoint::new(2, "b", 10, 100));
        v.start();
        v.advance(); // reaches a, completes (only one advance with 2 wps would go to b)
        // Actually advance() returns true when moving to next, false when finishing
        // With 2 waypoints: advance once -> reaches a, current=b. advance again -> reaches b, completes.
        // But we only advanced once so it's Active still
        let comp = VoyageCompletion::new(vec![WaypointId(2)], 0);
        let result = comp.verify(&v);
        assert!(!result.success);
    }

    #[test]
    fn test_completion_resources_insufficient() {
        let mut v = Voyage::new(1, "test", 100);
        v.add_waypoint(Waypoint::new(1, "a", 10, 100));
        v.start();
        v.advance(); // completes
        let comp = VoyageCompletion::new(vec![WaypointId(1)], 500);
        let result = comp.verify(&v);
        assert!(!result.resources_ok);
    }
}
